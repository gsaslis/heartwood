use std::path::Path;
use std::str::FromStr;
use std::{env, thread, time};

use radicle::git;
use radicle::node;
use radicle::node::address::Store as _;
use radicle::node::routing::Store as _;
use radicle::node::Handle as _;
use radicle::node::{Alias, DEFAULT_TIMEOUT};
use radicle::prelude::Id;
use radicle::profile::Home;
use radicle::storage::{ReadStorage, RemoteRepository};
use radicle::test::fixtures;

use radicle_cli_test::TestFormula;
use radicle_node::service::tracking::{Policy, Scope};
use radicle_node::service::Event;
use radicle_node::test::environment::{Config, Environment};
#[allow(unused_imports)]
use radicle_node::test::logger;

/// Seed used in tests.
const RAD_SEED: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

/// Run a CLI test file.
fn test<'a>(
    test: impl AsRef<Path>,
    cwd: impl AsRef<Path>,
    home: Option<&Home>,
    envs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir().unwrap();
    let home = if let Some(home) = home {
        home.path().to_path_buf()
    } else {
        tmp.path().to_path_buf()
    };

    formula(cwd.as_ref(), test)?
        .env("RAD_HOME", home.to_string_lossy())
        .envs(envs)
        .run()?;

    Ok(())
}

/// Test config.
fn config(alias: &'static str) -> Config {
    Config {
        external_addresses: vec![
            node::Address::from_str(&format!("{alias}.radicle.xyz:8776")).unwrap(),
        ],
        ..Config::test(Alias::new(alias))
    }
}

fn formula(root: &Path, test: impl AsRef<Path>) -> Result<TestFormula, Box<dyn std::error::Error>> {
    let mut formula = TestFormula::new(root.to_path_buf());
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));

    formula
        .env("GIT_AUTHOR_DATE", "1671125284")
        .env("GIT_AUTHOR_EMAIL", "radicle@localhost")
        .env("GIT_AUTHOR_NAME", "radicle")
        .env("GIT_COMMITTER_DATE", "1671125284")
        .env("GIT_COMMITTER_EMAIL", "radicle@localhost")
        .env("GIT_COMMITTER_NAME", "radicle")
        .env("RAD_PASSPHRASE", "radicle")
        .env("RAD_SEED", RAD_SEED)
        .env("RAD_RNG_SEED", "0")
        .env("EDITOR", "true")
        .env("TZ", "UTC")
        .env("LANG", "C")
        .env("USER", "alice")
        .env(radicle_cob::git::RAD_COMMIT_TIME, "1671125284")
        .envs(git::env::GIT_DEFAULT_CONFIG)
        .build(&[
            ("radicle-remote-helper", "git-remote-rad"),
            ("radicle-cli", "rad"),
        ])
        .file(base.join(test))?;

    Ok(formula)
}

#[test]
fn rad_auth() {
    test("examples/rad-auth.md", Path::new("."), None, []).unwrap();
}

#[test]
fn rad_issue() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let home = &profile.home;
    let working = environment.tmp().join("working");

    // Setup a test repository.
    fixtures::repository(&working);

    test("examples/rad-init.md", &working, Some(home), []).unwrap();
    test("examples/rad-issue.md", &working, Some(home), []).unwrap();
}

#[test]
fn rad_cob() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let home = &profile.home;
    let working = environment.tmp().join("working");

    // Setup a test repository.
    fixtures::repository(&working);

    test("examples/rad-init.md", &working, Some(home), []).unwrap();
    test("examples/rad-cob.md", &working, Some(home), []).unwrap();
}

#[test]
fn rad_label() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let home = &profile.home;
    let working = environment.tmp().join("working");

    // Setup a test repository.
    fixtures::repository(&working);

    test("examples/rad-init.md", &working, Some(home), []).unwrap();
    test("examples/rad-issue.md", &working, Some(home), []).unwrap();
    test("examples/rad-label.md", &working, Some(home), []).unwrap();
}

#[test]
fn rad_init() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();

    // Setup a test repository.
    fixtures::repository(working.path());

    test(
        "examples/rad-init.md",
        working.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_init_no_git() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();

    test(
        "examples/rad-init-no-git.md",
        working.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_inspect() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();

    // Setup a test repository.
    fixtures::repository(working.path());

    test(
        "examples/rad-init.md",
        working.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-inspect.md",
        working.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();

    test("examples/rad-inspect-noauth.md", working.path(), None, []).unwrap();
}

#[test]
fn rad_checkout() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let copy = tempfile::tempdir().unwrap();

    // Setup a test repository.
    fixtures::repository(working.path());

    test(
        "examples/rad-init.md",
        working.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-checkout.md",
        copy.path(),
        Some(&profile.home),
        [],
    )
    .unwrap();

    if cfg!(target_os = "linux") {
        test(
            "examples/rad-checkout-repo-config-linux.md",
            copy.path(),
            Some(&profile.home),
            [],
        )
        .unwrap();
    } else if cfg!(target_os = "macos") {
        test(
            "examples/rad-checkout-repo-config-macos.md",
            copy.path(),
            Some(&profile.home),
            [],
        )
        .unwrap();
    }
}

#[test]
fn rad_id() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test("examples/rad-id.md", working.path(), Some(home), []).unwrap();
}

#[test]
fn rad_id_multi_delegate() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = tempfile::tempdir().unwrap();
    let working = working.path();
    let acme = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();

    // Setup a test repository.
    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.handle.track_repo(acme, Scope::All).unwrap();
    alice.connect(&bob).converge([&bob]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    // TODO: Have formula with two connected nodes and a tracked project.

    formula(&environment.tmp(), "examples/rad-id-multi-delegate.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_id_conflict() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = tempfile::tempdir().unwrap();
    let working = working.path();
    let acme = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();

    // Setup a test repository.
    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.handle.track_repo(acme, Scope::All).unwrap();
    alice.connect(&bob).converge([&bob]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    formula(&environment.tmp(), "examples/rad-id-conflict.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_node_connect() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = tempfile::tempdir().unwrap();
    let alice = alice.spawn();
    let bob = bob.spawn();

    alice
        .rad(
            "node",
            &["connect", format!("{}@{}", bob.id, bob.addr).as_str()],
            working.path(),
        )
        .unwrap();

    let sessions = alice.handle.sessions().unwrap();
    let session = sessions.first().unwrap();

    assert_eq!(session.nid, bob.id);
    assert_eq!(session.addr, bob.addr.into());
    assert!(session.state.is_connected());
}

#[test]
fn rad_node() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = tempfile::tempdir().unwrap();
    let alice = alice.spawn();

    fixtures::repository(working.path().join("alice"));

    test(
        "examples/rad-init-sync.md",
        &working.path().join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-node.md",
        working.path().join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test("examples/rad-issue.md", working.path(), Some(home), []).unwrap();
    test("examples/rad-patch.md", working.path(), Some(home), []).unwrap();
}

#[test]
fn rad_patch_checkout() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-checkout.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch_checkout_revision() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-checkout.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
    test(
        "examples/rad-patch-checkout-revision.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch_checkout_force() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");
    let acme = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();

    // Setup a test repository.
    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.handle.track_repo(acme, Scope::All).unwrap();
    alice.connect(&bob).converge([&bob]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    formula(&environment.tmp(), "examples/rad-patch-checkout-force.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_patch_update() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-update.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
#[cfg(not(target_os = "macos"))]
fn rad_patch_ahead_behind() {
    use std::fs;

    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    fs::write(working.path().join("CONTRIBUTORS"), "Alice Jones\n").unwrap();

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-ahead-behind.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch_draft() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-draft.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch_via_push() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-patch-via-push.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
#[cfg(not(target_os = "macos"))]
fn rad_review_by_hunk() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test(
        "examples/rad-review-by-hunk.md",
        working.path(),
        Some(home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_rm() {
    let mut environment = Environment::new();
    let profile = environment.profile("alice");
    let working = tempfile::tempdir().unwrap();
    let home = &profile.home;

    // Setup a test repository.
    fixtures::repository(working.path());

    test("examples/rad-init.md", working.path(), Some(home), []).unwrap();
    test("examples/rad-rm.md", working.path(), Some(home), []).unwrap();
}

#[test]
fn rad_track() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = tempfile::tempdir().unwrap();
    let alice = alice.spawn();

    test(
        "examples/rad-track.md",
        working.path(),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_clone() {
    let mut environment = Environment::new();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    // Setup a test project.
    let acme = alice.project("heartwood", "Radicle Heartwood Protocol & Stack");

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    // Prevent Alice from fetching Bob's fork, as we're not testing that and it may cause errors.
    alice.handle.track_repo(acme, Scope::Trusted).unwrap();

    bob.connect(&alice).converge([&alice]);

    test("examples/rad-clone.md", working, Some(&bob.home), []).unwrap();
}

#[test]
fn rad_clone_all() {
    let mut environment = Environment::new();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let eve = environment.node(Config::test(Alias::new("eve")));
    let working = environment.tmp().join("working");

    // Setup a test project.
    let acme = alice.project("heartwood", "Radicle Heartwood Protocol & Stack");

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    let mut eve = eve.spawn();

    alice.handle.track_repo(acme, Scope::All).unwrap();
    bob.connect(&alice).converge([&alice]);
    eve.connect(&alice).converge([&alice]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
    bob.has_inventory_of(&acme, &alice.id);
    alice.has_inventory_of(&acme, &bob.id);

    test(
        "examples/rad-clone-all.md",
        working.join("eve"),
        Some(&eve.home),
        [],
    )
    .unwrap();
    eve.has_inventory_of(&acme, &bob.id);
}

#[test]
fn rad_clone_connect() {
    let mut environment = Environment::new();
    let working = environment.tmp().join("working");
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let mut eve = environment.node(Config::test(Alias::new("eve")));
    let acme = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();
    let now = localtime::LocalTime::now().as_secs();

    fixtures::repository(working.join("acme"));

    test(
        "examples/rad-init.md",
        working.join("acme"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();

    // Let Eve know about Alice and Bob having the repo.
    eve.routing.insert([&acme], alice.id, now).unwrap();
    eve.routing.insert([&acme], bob.id, now).unwrap();
    eve.addresses
        .insert(
            &alice.id,
            node::Features::SEED,
            Alias::new("alice"),
            0,
            now,
            [node::KnownAddress::new(
                node::Address::from(alice.addr),
                node::address::Source::Imported,
            )],
        )
        .unwrap();
    eve.addresses
        .insert(
            &bob.id,
            node::Features::SEED,
            Alias::new("bob"),
            0,
            now,
            [node::KnownAddress::new(
                node::Address::from(bob.addr),
                node::address::Source::Imported,
            )],
        )
        .unwrap();
    eve.config.peers = node::config::PeerConfig::Static;

    let eve = eve.spawn();

    alice.handle.track_repo(acme, Scope::Trusted).unwrap();
    bob.handle.track_repo(acme, Scope::Trusted).unwrap();
    alice.connect(&bob);
    bob.routes_to(&[(acme, alice.id)]);
    eve.routes_to(&[(acme, alice.id), (acme, bob.id)]);
    alice.routes_to(&[(acme, alice.id), (acme, bob.id)]);

    test(
        "examples/rad-clone-connect.md",
        working.join("acme"),
        Some(&eve.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_sync_without_node() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let mut eve = environment.node(Config::test(Alias::new("eve")));

    let rid = Id::from_urn("rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5").unwrap();
    eve.tracking.track_repo(&rid, Scope::All).unwrap();

    formula(&environment.tmp(), "examples/rad-sync-without-node.md")
        .unwrap()
        .home(
            "alice",
            alice.home.path(),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            bob.home.path(),
            [("RAD_HOME", bob.home.path().display())],
        )
        .home(
            "eve",
            eve.home.path(),
            [("RAD_HOME", eve.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_self() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    test("examples/rad-self.md", working, Some(&alice.home), []).unwrap();
}

#[test]
fn rad_clone_unknown() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    let alice = alice.spawn();

    test(
        "examples/rad-clone-unknown.md",
        working,
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_init_sync_and_clone() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice);

    fixtures::repository(working.join("alice"));

    // Necessary for now, if we don't want the new inventry announcement to be considered stale
    // for Bob.
    // TODO: Find a way to advance internal clocks instead.
    thread::sleep(time::Duration::from_millis(3));

    // Alice initializes a repo after her node has started, and after bob has connected to it.
    test(
        "examples/rad-init-sync.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    // Wait for bob to get any updates to the routing table.
    bob.converge([&alice]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_fetch() {
    let mut environment = Environment::new();
    let working = environment.tmp().join("working");
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));

    let mut alice = alice.spawn();
    let bob = bob.spawn();

    alice.connect(&bob);
    fixtures::repository(working.join("alice"));

    // Alice initializes a repo after her node has started, and after bob has connected to it.
    test(
        "examples/rad-init-sync.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    // Wait for bob to get any updates to the routing table.
    bob.converge([&alice]);

    test(
        "examples/rad-fetch.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_fork() {
    let mut environment = Environment::new();
    let working = environment.tmp().join("working");
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));

    let mut alice = alice.spawn();
    let bob = bob.spawn();

    alice.connect(&bob);
    fixtures::repository(working.join("alice"));

    // Alice initializes a repo after her node has started, and after bob has connected to it.
    test(
        "examples/rad-init-sync.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    // Wait for bob to get any updates to the routing table.
    bob.converge([&alice]);

    test(
        "examples/rad-fetch.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
    test(
        "examples/rad-fork.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_diff() {
    let working = tempfile::tempdir().unwrap();

    fixtures::repository(&working);

    test("examples/rad-diff.md", working, None, []).unwrap();
}

#[test]
// User tries to clone; no seeds are available, but user has the repo locally.
fn test_clone_without_seeds() {
    let mut environment = Environment::new();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");
    let rid = alice.project("heartwood", "Radicle Heartwood Protocol & Stack");
    let mut alice = alice.spawn();
    let seeds = alice.handle.seeds(rid).unwrap();
    let connected = seeds.connected().collect::<Vec<_>>();

    assert!(connected.is_empty());

    alice
        .rad("clone", &[rid.to_string().as_str()], working.as_path())
        .unwrap();

    alice
        .rad("inspect", &[], working.join("heartwood").as_path())
        .unwrap();
}

#[test]
fn test_cob_replication() {
    let mut environment = Environment::new();
    let working = tempfile::tempdir().unwrap();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));

    let rid = alice.project("heartwood", "");

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    let events = alice.handle.events();

    alice.handle.track_node(bob.id, None).unwrap();
    alice.connect(&bob);

    bob.routes_to(&[(rid, alice.id)]);
    bob.rad("clone", &[rid.to_string().as_str()], working.path())
        .unwrap();

    // Wait for Alice to fetch the clone refs.
    events
        .wait(
            |e| matches!(e, Event::RefsFetched { .. }).then_some(()),
            time::Duration::from_secs(6),
        )
        .unwrap();

    let bob_repo = bob.storage.repository(rid).unwrap();
    let mut bob_issues = radicle::cob::issue::Issues::open(&bob_repo).unwrap();
    let issue = bob_issues
        .create(
            "Something's fishy",
            "I don't know what it is",
            &[],
            &[],
            [],
            &bob.signer,
        )
        .unwrap();
    log::debug!(target: "test", "Issue {} created", issue.id());

    // Make sure that Bob's issue refs announcement has a different timestamp than his fork's
    // announcement, otherwise Alice will consider it stale.
    thread::sleep(time::Duration::from_millis(3));

    bob.handle.announce_refs(rid).unwrap();

    // Wait for Alice to fetch the issue refs.
    events
        .iter()
        .find(|e| matches!(e, Event::RefsFetched { .. }))
        .unwrap();

    let alice_repo = alice.storage.repository(rid).unwrap();
    let alice_issues = radicle::cob::issue::Issues::open(&alice_repo).unwrap();
    let alice_issue = alice_issues.get(issue.id()).unwrap().unwrap();

    assert_eq!(alice_issue.title(), "Something's fishy");
}

#[test]
fn test_cob_deletion() {
    let mut environment = Environment::new();
    let working = tempfile::tempdir().unwrap();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));

    let rid = alice.project("heartwood", "");

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();

    alice.handle.track_repo(rid, Scope::All).unwrap();
    bob.handle.track_repo(rid, Scope::All).unwrap();
    alice.connect(&bob);
    bob.routes_to(&[(rid, alice.id)]);

    let alice_repo = alice.storage.repository(rid).unwrap();
    let mut alice_issues = radicle::cob::issue::Issues::open(&alice_repo).unwrap();
    let issue = alice_issues
        .create(
            "Something's fishy",
            "I don't know what it is",
            &[],
            &[],
            [],
            &alice.signer,
        )
        .unwrap();
    let issue_id = issue.id();
    log::debug!(target: "test", "Issue {} created", issue_id);

    bob.rad("clone", &[rid.to_string().as_str()], working.path())
        .unwrap();

    let bob_repo = bob.storage.repository(rid).unwrap();
    let bob_issues = radicle::cob::issue::Issues::open(&bob_repo).unwrap();
    assert!(bob_issues.get(issue_id).unwrap().is_some());

    let alice_issues = radicle::cob::issue::Issues::open(&alice_repo).unwrap();
    alice_issues.remove(issue_id, &alice.signer).unwrap();

    log::debug!(target: "test", "Removing issue..");

    radicle::assert_matches!(
        bob.handle.fetch(rid, alice.id, DEFAULT_TIMEOUT).unwrap(),
        radicle::node::FetchResult::Success { .. }
    );
    let bob_repo = bob.storage.repository(rid).unwrap();
    let bob_issues = radicle::cob::issue::Issues::open(&bob_repo).unwrap();
    assert!(bob_issues.get(issue_id).unwrap().is_none());
}

#[test]
fn rad_sync() {
    let mut environment = Environment::new();
    let working = environment.tmp().join("working");
    let alice = environment.node(config("alice"));
    let bob = environment.node(config("bob"));
    let eve = environment.node(config("eve"));
    let acme = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();

    fixtures::repository(working.join("acme"));

    test(
        "examples/rad-init.md",
        working.join("acme"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    let mut eve = eve.spawn();

    bob.handle.track_repo(acme, Scope::All).unwrap();
    eve.handle.track_repo(acme, Scope::All).unwrap();

    alice.connect(&bob);
    eve.connect(&alice);

    bob.routes_to(&[(acme, alice.id)]);
    eve.routes_to(&[(acme, alice.id)]);
    alice.routes_to(&[(acme, alice.id), (acme, eve.id), (acme, bob.id)]);

    test(
        "examples/rad-sync.md",
        working.join("acme"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
//
//     alice -- seed -- bob
//
fn test_replication_via_seed() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let seed = environment.node(Config {
        policy: Policy::Track,
        scope: Scope::All,
        ..Config::test(Alias::new("seed"))
    });
    let working = environment.tmp().join("working");
    let rid = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    let seed = seed.spawn();

    alice.connect(&seed);
    bob.connect(&seed);

    // Enough time for the next inventory from Seed to not be considered stale by Bob.
    thread::sleep(time::Duration::from_millis(3));

    alice.routes_to(&[]);
    seed.routes_to(&[]);
    bob.routes_to(&[]);

    // Initialize a repo as Alice.
    fixtures::repository(working.join("alice"));
    alice
        .rad(
            "init",
            &[
                "--name",
                "heartwood",
                "--description",
                "Radicle Heartwood Protocol & Stack",
                "--default-branch",
                "master",
                "--public",
            ],
            working.join("alice"),
        )
        .unwrap();

    alice
        .rad("track", &[&bob.id.to_human()], working.join("alice"))
        .unwrap();

    alice.routes_to(&[(rid, alice.id), (rid, seed.id)]);
    seed.routes_to(&[(rid, alice.id), (rid, seed.id)]);
    bob.routes_to(&[(rid, alice.id), (rid, seed.id)]);

    let seed_events = seed.handle.events();
    let alice_events = alice.handle.events();

    bob.rad("clone", &[rid.to_string().as_str()], working.join("bob"))
        .unwrap();

    alice.routes_to(&[(rid, alice.id), (rid, seed.id), (rid, bob.id)]);
    seed.routes_to(&[(rid, alice.id), (rid, seed.id), (rid, bob.id)]);
    bob.routes_to(&[(rid, alice.id), (rid, seed.id), (rid, bob.id)]);

    seed_events
        .iter()
        .any(|e| matches!(e, Event::RefsFetched { remote, .. } if remote == bob.id));
    alice_events
        .iter()
        .any(|e| matches!(e, Event::RefsFetched { remote, .. } if remote == seed.id));

    seed.storage
        .repository(rid)
        .unwrap()
        .remote(&bob.id)
        .unwrap();

    // Seed should send Bob's ref announcement to Alice, after the fetch.
    alice
        .storage
        .repository(rid)
        .unwrap()
        .remote(&bob.id)
        .unwrap();
}

#[test]
fn rad_remote() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");
    let home = alice.home.clone();
    let rid = Id::from_str("z42hL2jL4XNk6K8oHQaSWfMgCL7ji").unwrap();
    // Setup a test repository.
    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&home),
        [],
    )
    .unwrap();

    let mut alice = alice.spawn();
    let mut bob = bob.spawn();
    alice
        .handle
        .track_node(bob.id, Some(Alias::new("bob")))
        .unwrap();

    bob.connect(&alice);
    bob.routes_to(&[(rid, alice.id)]);
    bob.rad("clone", &[rid.to_string().as_str()], &working)
        .unwrap();

    alice.has_inventory_of(&rid, &bob.id);

    test(
        "examples/rad-remote.md",
        working.join("alice"),
        Some(&home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_merge_via_push() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();

    test(
        "examples/rad-merge-via-push.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_merge_after_update() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();

    test(
        "examples/rad-merge-after-update.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_merge_no_ff() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-merge-no-ff.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_patch_pull_update() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);

    formula(&environment.tmp(), "examples/rad-patch-pull-update.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            bob.home.path(),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_init_private() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init-private.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_init_private_clone() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    test(
        "examples/rad-init-private.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    bob.connect(&alice).converge([&alice]);

    formula(&environment.tmp(), "examples/rad-init-private-clone.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            bob.home.path(),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_publish() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init-private.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-publish.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn framework_home() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));

    formula(&environment.tmp(), "examples/framework/home.md")
        .unwrap()
        .home(
            "alice",
            alice.home.path(),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            bob.home.path(),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn git_push_diverge() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    formula(&environment.tmp(), "examples/git/git-push-diverge.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob").join("heartwood"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_push_and_pull_patches() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);

    logger::init(log::Level::Debug);

    test(
        "examples/rad-clone.md",
        working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    formula(&environment.tmp(), "examples/rad-push-and-pull-patches.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob").join("heartwood"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_patch_fetch_1() {
    let mut environment = Environment::new();
    let mut alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");
    let (repo, _) = fixtures::repository(working.join("alice"));
    let rid = alice.project_from("heartwood", "Radicle Heartwood Protocol & Stack", &repo);

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);
    bob.clone(rid, working.join("bob")).unwrap();

    formula(&environment.tmp(), "examples/rad-patch-fetch-1.md")
        .unwrap()
        .home(
            "alice",
            working.join("alice"),
            [("RAD_HOME", alice.home.path().display())],
        )
        .home(
            "bob",
            working.join("bob").join("heartwood"),
            [("RAD_HOME", bob.home.path().display())],
        )
        .run()
        .unwrap();
}

#[test]
fn rad_patch_fetch_2() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    test(
        "examples/rad-patch-fetch-2.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn git_push_and_fetch() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/rad-init.md",
        working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);

    test(
        "examples/rad-clone.md",
        &working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
    test(
        "examples/git/git-push.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
    test(
        "examples/git/git-fetch.md",
        &working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();
    test(
        "examples/git/git-push-delete.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();
}

#[test]
fn rad_workflow() {
    let mut environment = Environment::new();
    let alice = environment.node(Config::test(Alias::new("alice")));
    let bob = environment.node(Config::test(Alias::new("bob")));
    let working = environment.tmp().join("working");

    fixtures::repository(working.join("alice"));

    test(
        "examples/workflow/1-new-project.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    let alice = alice.spawn();
    let mut bob = bob.spawn();

    bob.connect(&alice).converge([&alice]);

    test(
        "examples/workflow/2-cloning.md",
        &working.join("bob"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    test(
        "examples/workflow/3-issues.md",
        &working.join("bob").join("heartwood"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    test(
        "examples/workflow/4-patching-contributor.md",
        &working.join("bob").join("heartwood"),
        Some(&bob.home),
        [],
    )
    .unwrap();

    test(
        "examples/workflow/5-patching-maintainer.md",
        &working.join("alice"),
        Some(&alice.home),
        [],
    )
    .unwrap();

    test(
        "examples/workflow/6-pulling-contributor.md",
        &working.join("bob").join("heartwood"),
        Some(&bob.home),
        [],
    )
    .unwrap();
}
