pub mod git;
pub mod refs;

use std::collections::{hash_map, HashSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{fmt, io};

use nonempty::NonEmpty;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crypto::{PublicKey, Signer, Unverified, Verified};
pub use git::VerifyError;
pub use radicle_git_ext::Oid;

use crate::collections::HashMap;
use crate::git::ext as git_ext;
use crate::git::{Qualified, RefError, RefString};
use crate::identity;
use crate::identity::doc::DocError;
use crate::identity::Did;
use crate::identity::{Id, IdentityError};
use crate::storage::refs::Refs;

use self::refs::SignedRefs;

pub type BranchName = git::RefString;
pub type Inventory = Vec<Id>;

/// Describes one or more namespaces.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Namespaces {
    /// All namespaces.
    #[default]
    All,
    /// The trusted set of namespaces.
    Trusted(HashSet<PublicKey>),
}

impl FromIterator<PublicKey> for Namespaces {
    fn from_iter<T: IntoIterator<Item = PublicKey>>(iter: T) -> Self {
        Self::Trusted(iter.into_iter().collect())
    }
}

/// Storage error.
#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid git reference")]
    InvalidRef,
    #[error("identity doc: {0}")]
    Doc(#[from] DocError),
    #[error("git reference error: {0}")]
    Ref(#[from] RefError),
    #[error(transparent)]
    Refs(#[from] refs::Error),
    #[error("git: {0}")]
    Git(#[from] git2::Error),
    #[error("invalid repository identifier {0:?}")]
    InvalidId(std::ffi::OsString),
    #[error("i/o: {0}")]
    Io(#[from] io::Error),
}

impl Error {
    /// Whether this error is caused by something not being found.
    pub fn is_not_found(&self) -> bool {
        match self {
            Self::Io(e) if e.kind() == io::ErrorKind::NotFound => true,
            Self::Git(e) if git::is_not_found_err(e) => true,
            Self::Doc(e) if e.is_not_found() => true,
            _ => false,
        }
    }
}

/// Fetch error.
#[derive(Error, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum FetchError {
    #[error("git: {0}")]
    Git(#[from] git2::Error),
    #[error("i/o: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Refs(#[from] refs::Error),
    #[error("verify: {0}")]
    Verify(#[from] git::VerifyError),
    #[error(transparent)]
    Storage(#[from] Error),
    // TODO: This should wrap a more specific error.
    #[error("repository head: {0}")]
    SetHead(#[from] IdentityError),
}

pub type RemoteId = PublicKey;

/// An update to a reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefUpdate {
    Updated { name: RefString, old: Oid, new: Oid },
    Created { name: RefString, oid: Oid },
    Deleted { name: RefString, oid: Oid },
    Skipped { name: RefString, oid: Oid },
}

impl RefUpdate {
    pub fn from(name: RefString, old: impl Into<Oid>, new: impl Into<Oid>) -> Self {
        let old = old.into();
        let new = new.into();

        if old.is_zero() {
            Self::Created { name, oid: new }
        } else if new.is_zero() {
            Self::Deleted { name, oid: old }
        } else if old != new {
            Self::Updated { name, old, new }
        } else {
            Self::Skipped { name, oid: old }
        }
    }
}

impl fmt::Display for RefUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Updated { name, old, new } => {
                write!(f, "~ {old:.7}..{new:.7} {name}")
            }
            Self::Created { name, oid } => {
                write!(f, "* 0000000..{oid:.7} {name}")
            }
            Self::Deleted { name, oid } => {
                write!(f, "- {oid:.7}..0000000 {name}")
            }
            Self::Skipped { name, oid } => {
                write!(f, "= {oid:.7}..{oid:.7} {name}")
            }
        }
    }
}

/// Project remotes. Tracks the git state of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remotes<V>(HashMap<RemoteId, Remote<V>>);

impl<V> FromIterator<(RemoteId, Remote<V>)> for Remotes<V> {
    fn from_iter<T: IntoIterator<Item = (RemoteId, Remote<V>)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V> Deref for Remotes<V> {
    type Target = HashMap<RemoteId, Remote<V>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Remotes<V> {
    pub fn new(remotes: HashMap<RemoteId, Remote<V>>) -> Self {
        Self(remotes)
    }
}

impl Remotes<Verified> {
    pub fn unverified(self) -> Remotes<Unverified> {
        Remotes(
            self.into_iter()
                .map(|(id, r)| (id, r.unverified()))
                .collect(),
        )
    }
}

impl<V> Default for Remotes<V> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl<V> IntoIterator for Remotes<V> {
    type Item = (RemoteId, Remote<V>);
    type IntoIter = hash_map::IntoIter<RemoteId, Remote<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<V> From<Remotes<V>> for HashMap<RemoteId, Refs> {
    fn from(other: Remotes<V>) -> Self {
        let mut remotes = HashMap::with_hasher(fastrand::Rng::new().into());

        for (k, v) in other.into_iter() {
            remotes.insert(k, v.refs.into());
        }
        remotes
    }
}

/// A project remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Remote<V = Verified> {
    /// Git references published under this remote, and their hashes.
    #[serde(flatten)]
    pub refs: SignedRefs<V>,
}

impl Remote<Unverified> {
    /// Create a new unverified remotes object.
    pub fn new(refs: impl Into<SignedRefs<Unverified>>) -> Self {
        Self { refs: refs.into() }
    }
}

impl Remote<Unverified> {
    pub fn verified(self) -> Result<Remote<Verified>, crypto::Error> {
        let refs = self.refs.verified()?;

        Ok(Remote { refs })
    }
}

impl Remote<Verified> {
    /// Create a new unverified remotes object.
    pub fn new(refs: impl Into<SignedRefs<Verified>>) -> Self {
        Self { refs: refs.into() }
    }

    pub fn unverified(self) -> Remote<Unverified> {
        Remote {
            refs: self.refs.unverified(),
        }
    }
}

impl<V> Deref for Remote<V> {
    type Target = SignedRefs<V>;

    fn deref(&self) -> &Self::Target {
        &self.refs
    }
}

/// Read-only operations on a storage instance.
pub trait ReadStorage {
    type Repository: ReadRepository;

    /// Get the storage base path.
    fn path(&self) -> &Path;
    /// Get a repository's path.
    fn path_of(&self, rid: &Id) -> PathBuf;
    /// Get an identity document of a repository under a given remote.
    fn get(
        &self,
        remote: &RemoteId,
        rid: Id,
    ) -> Result<Option<identity::Doc<Verified>>, IdentityError>;
    /// Check whether storage contains a repository.
    fn contains(&self, rid: &Id) -> Result<bool, IdentityError>;
    /// Get the inventory of repositories hosted under this storage.
    fn inventory(&self) -> Result<Inventory, Error>;
    /// Open or create a read-only repository.
    fn repository(&self, rid: Id) -> Result<Self::Repository, Error>;
}

/// Allows access to individual storage repositories.
pub trait WriteStorage: ReadStorage {
    type RepositoryMut: WriteRepository;

    /// Open a read-write repository.
    fn repository_mut(&self, rid: Id) -> Result<Self::RepositoryMut, Error>;
    /// Create a read-write repository.
    fn create(&self, rid: Id) -> Result<Self::RepositoryMut, Error>;
}

/// Allows read-only access to a repository.
pub trait ReadRepository {
    /// Return the repository id.
    fn id(&self) -> Id;

    /// Returns `true` if there are no references in the repository.
    fn is_empty(&self) -> Result<bool, git2::Error>;

    /// The [`Path`] to the git repository.
    fn path(&self) -> &Path;

    /// Get a blob in this repository at the given commit and path.
    fn blob_at<'a>(&'a self, commit: Oid, path: &'a Path)
        -> Result<git2::Blob<'a>, git_ext::Error>;

    /// Validate all remotes with [`ReadRepository::validate_remote`].
    fn validate(&self) -> Result<(), VerifyError> {
        for (_, remote) in self.remotes()? {
            self.validate_remote(&remote)?;
        }
        Ok(())
    }

    /// Validates a remote's signed refs and identity.
    ///
    /// Returns any ref found under that remote that isn't signed.
    /// If a signed ref is missing from the repository, an error is returned.
    fn validate_remote(&self, remote: &Remote<Verified>) -> Result<Vec<RefString>, VerifyError>;

    /// Get the head of this repository.
    ///
    /// Returns the reference pointed to by `HEAD` if it is set. Otherwise, computes the canonical
    /// head using [`ReadRepository::canonical_head`].
    ///
    /// Returns the [`Oid`] as well as the qualified reference name.
    fn head(&self) -> Result<(Qualified, Oid), IdentityError>;

    /// Compute the canonical head of this repository.
    ///
    /// Ignores any existing `HEAD` reference.
    ///
    /// Returns the [`Oid`] as well as the qualified reference name.
    fn canonical_head(&self) -> Result<(Qualified, Oid), IdentityError>;

    /// Get the head of the `rad/id` reference in this repository.
    ///
    /// Returns the reference pointed to by `rad/id` if it is set. Otherwise, computes the canonical
    /// `rad/id` using [`ReadRepository::canonical_identity_head`].
    fn identity_head(&self) -> Result<Oid, IdentityError>;

    /// Compute the canonical `rad/id` of this repository.
    ///
    /// Ignores any existing `rad/id` reference.
    fn canonical_identity_head(&self) -> Result<Oid, IdentityError>;

    /// Get the `reference` for the given `remote`.
    ///
    /// Returns `None` is the reference did not exist.
    fn reference(
        &self,
        remote: &RemoteId,
        reference: &Qualified,
    ) -> Result<git2::Reference, git_ext::Error>;

    /// Get the [`git2::Commit`] found using its `oid`.
    ///
    /// Returns `None` if the commit did not exist.
    fn commit(&self, oid: Oid) -> Result<git2::Commit, git_ext::Error>;

    /// Perform a revision walk of a commit history starting from the given head.
    fn revwalk(&self, head: Oid) -> Result<git2::Revwalk, git2::Error>;

    /// Get the object id of a reference under the given remote.
    fn reference_oid(
        &self,
        remote: &RemoteId,
        reference: &Qualified,
    ) -> Result<Oid, git_ext::Error>;

    /// Get all references of the given remote.
    fn references_of(&self, remote: &RemoteId) -> Result<Refs, Error>;

    /// Get the given remote.
    fn remote(&self, remote: &RemoteId) -> Result<Remote<Verified>, refs::Error>;

    /// Get all remotes.
    fn remotes(&self) -> Result<Remotes<Verified>, refs::Error>;

    /// Get repository delegates.
    fn delegates(&self) -> Result<NonEmpty<Did>, IdentityError> {
        let (_, doc) = self.identity_doc()?;
        let doc = doc.verified()?;

        Ok(doc.delegates)
    }

    /// Get the repository's identity document.
    fn identity_doc(&self) -> Result<(Oid, identity::Doc<Unverified>), IdentityError> {
        let head = self.identity_head()?;
        let doc = self.identity_doc_at(head)?;

        Ok((head, doc))
    }

    /// Get the repository's identity document at a specific commit.
    fn identity_doc_at(&self, head: Oid) -> Result<identity::Doc<Unverified>, DocError>;
}

/// Allows read-write access to a repository.
pub trait WriteRepository: ReadRepository {
    /// Set the repository head to the canonical branch.
    /// This computes the head based on the delegate set.
    fn set_head(&self) -> Result<Oid, IdentityError>;
    /// Set the repository 'rad/id' to the canonical commit, agreed by quorum.
    fn set_identity_head(&self) -> Result<Oid, IdentityError>;
    /// Sign the repository's refs under the `refs/rad/sigrefs` branch.
    fn sign_refs<G: Signer>(&self, signer: &G) -> Result<SignedRefs<Verified>, Error>;
    /// Get the underlying git repository.
    fn raw(&self) -> &git2::Repository;
}

impl<T, S> ReadStorage for T
where
    T: Deref<Target = S>,
    S: ReadStorage + 'static,
{
    type Repository = S::Repository;

    fn path(&self) -> &Path {
        self.deref().path()
    }

    fn path_of(&self, rid: &Id) -> PathBuf {
        self.deref().path_of(rid)
    }

    fn contains(&self, rid: &Id) -> Result<bool, IdentityError> {
        self.deref().contains(rid)
    }

    fn inventory(&self) -> Result<Inventory, Error> {
        self.deref().inventory()
    }

    fn get(
        &self,
        remote: &RemoteId,
        proj: Id,
    ) -> Result<Option<identity::Doc<Verified>>, IdentityError> {
        self.deref().get(remote, proj)
    }

    fn repository(&self, rid: Id) -> Result<Self::Repository, Error> {
        self.deref().repository(rid)
    }
}

impl<T, S> WriteStorage for T
where
    T: Deref<Target = S>,
    S: WriteStorage + 'static,
{
    type RepositoryMut = S::RepositoryMut;

    fn repository_mut(&self, rid: Id) -> Result<Self::RepositoryMut, Error> {
        self.deref().repository_mut(rid)
    }

    fn create(&self, rid: Id) -> Result<Self::RepositoryMut, Error> {
        self.deref().create(rid)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_storage() {}
}
