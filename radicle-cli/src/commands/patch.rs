#[path = "patch/archive.rs"]
mod archive;
#[path = "patch/checkout.rs"]
mod checkout;
#[path = "patch/common.rs"]
mod common;
#[path = "patch/create.rs"]
mod create;
#[path = "patch/delete.rs"]
mod delete;
#[path = "patch/list.rs"]
mod list;
#[path = "patch/ready.rs"]
mod ready;
#[path = "patch/show.rs"]
mod show;
#[path = "patch/update.rs"]
mod update;

use std::ffi::OsString;

use anyhow::anyhow;

use radicle::cob::patch;
use radicle::cob::patch::PatchId;
use radicle::storage::git::transport;
use radicle::{prelude::*, Node};

use crate::commands::rad_sync as sync;
use crate::git::Rev;
use crate::terminal as term;
use crate::terminal::args::{string, Args, Error, Help};
use crate::terminal::patch::Message;

pub const HELP: Help = Help {
    name: "patch",
    description: "Manage patches",
    version: env!("CARGO_PKG_VERSION"),
    usage: r#"
Usage

    rad patch [<option>...]
    rad patch list [--all|--merged|--open|--archived|--draft] [<option>...]
    rad patch show <patch-id> [<option>...]
    rad patch open [--draft] [<option>...]
    rad patch archive <patch-id> [<option>...]
    rad patch update <patch-id> [<option>...]
    rad patch checkout <patch-id> [<option>...]
    rad patch delete <patch-id> [<option>...]
    rad patch ready <patch-id> [--undo] [<option>...]

Show options

    -p, --patch                Show the actual patch diff

Open/Update options

        --draft                Open patch in draft mode
    -q, --quiet                Supress most output, only print the revision id
        --[no-]announce        Announce patch to network (default: false)
        --[no-]push            Push patch head to storage (default: true)
    -m, --message [<string>]   Provide a comment message to the patch or revision (default: prompt)
        --no-message           Leave the patch or revision comment message blank

List options

        --all                  Show all patches, including merged and archived patches
        --archived             Show only archived patches
        --merged               Show only merged patches
        --open                 Show only open patches (default)
        --draft                Show only draft patches

Ready options

        --undo                 Convert a patch back to a draft

Other options

        --help                 Print help
"#,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub enum OperationName {
    Open,
    Show,
    Update,
    Archive,
    Delete,
    Checkout,
    Ready,
    #[default]
    List,
}

#[derive(Debug)]
pub enum Operation {
    Open {
        message: Message,
        draft: bool,
        quiet: bool,
    },
    Show {
        patch_id: Rev,
        diff: bool,
    },
    Update {
        patch_id: Option<Rev>,
        message: Message,
        quiet: bool,
    },
    Archive {
        patch_id: Rev,
    },
    Ready {
        patch_id: Rev,
        undo: bool,
    },
    Delete {
        patch_id: Rev,
    },
    Checkout {
        patch_id: Rev,
    },
    List {
        filter: Option<patch::State>,
    },
}

#[derive(Debug)]
pub struct Options {
    pub op: Operation,
    pub fetch: bool,
    pub announce: bool,
    pub push: bool,
    pub verbose: bool,
}

impl Args for Options {
    fn from_args(args: Vec<OsString>) -> anyhow::Result<(Self, Vec<OsString>)> {
        use lexopt::prelude::*;

        let mut parser = lexopt::Parser::from_args(args);
        let mut op: Option<OperationName> = None;
        let mut verbose = false;
        let mut fetch = false;
        let mut announce = false;
        let mut patch_id = None;
        let mut message = Message::default();
        let mut push = true;
        let mut filter = Some(patch::State::Open);
        let mut diff = false;
        let mut draft = false;
        let mut undo = false;
        let mut quiet = false;

        while let Some(arg) = parser.next()? {
            match arg {
                // Options.
                Long("message") | Short('m') => {
                    if message != Message::Blank {
                        // We skip this code when `no-message` is specified.
                        let txt: String = term::args::string(&parser.value()?);
                        message.append(&txt);
                    }
                }
                Long("no-message") => {
                    message = Message::Blank;
                }
                Long("fetch") => {
                    fetch = true;
                }
                Long("no-fetch") => {
                    fetch = false;
                }
                Long("announce") => {
                    announce = true;
                }
                Long("no-announce") => {
                    announce = false;
                }
                Long("push") => {
                    push = true;
                }
                Long("no-push") => {
                    push = false;
                }

                // Open/update options.
                Long("draft") if op == Some(OperationName::Open) => {
                    draft = true;
                }
                Long("quiet") | Short('q')
                    if op == Some(OperationName::Open) || op == Some(OperationName::Update) =>
                {
                    quiet = true;
                }

                // Show options.
                Long("patch") | Short('p') if op == Some(OperationName::Show) => {
                    diff = true;
                }

                // Ready options.
                Long("undo") if op == Some(OperationName::Ready) => {
                    undo = true;
                }

                // List options.
                Long("all") => {
                    filter = None;
                }
                Long("draft") => {
                    filter = Some(patch::State::Draft);
                }
                Long("archived") => {
                    filter = Some(patch::State::Archived);
                }
                Long("merged") => {
                    filter = Some(patch::State::Merged);
                }
                Long("open") => {
                    filter = Some(patch::State::Open);
                }

                // Common.
                Long("verbose") | Short('v') => {
                    verbose = true;
                }
                Long("help") => {
                    return Err(Error::Help.into());
                }

                Value(val) if op.is_none() => match val.to_string_lossy().as_ref() {
                    "l" | "list" => op = Some(OperationName::List),
                    "o" | "open" => op = Some(OperationName::Open),
                    "s" | "show" => op = Some(OperationName::Show),
                    "u" | "update" => op = Some(OperationName::Update),
                    "d" | "delete" => op = Some(OperationName::Delete),
                    "c" | "checkout" => op = Some(OperationName::Checkout),
                    "a" | "archive" => op = Some(OperationName::Archive),
                    "y" | "ready" => op = Some(OperationName::Ready),
                    unknown => anyhow::bail!("unknown operation '{}'", unknown),
                },
                Value(val)
                    if patch_id.is_none()
                        && [
                            Some(OperationName::Show),
                            Some(OperationName::Update),
                            Some(OperationName::Delete),
                            Some(OperationName::Archive),
                            Some(OperationName::Ready),
                            Some(OperationName::Checkout),
                        ]
                        .contains(&op) =>
                {
                    let val = string(&val);
                    patch_id = Some(Rev::from(val));
                }
                _ => return Err(anyhow::anyhow!(arg.unexpected())),
            }
        }

        let op = match op.unwrap_or_default() {
            OperationName::Open => Operation::Open {
                message,
                draft,
                quiet,
            },
            OperationName::List => Operation::List { filter },
            OperationName::Show => Operation::Show {
                patch_id: patch_id.ok_or_else(|| anyhow!("a patch must be provided"))?,
                diff,
            },
            OperationName::Delete => Operation::Delete {
                patch_id: patch_id.ok_or_else(|| anyhow!("a patch must be provided"))?,
            },
            OperationName::Update => Operation::Update {
                patch_id,
                message,
                quiet,
            },
            OperationName::Archive => Operation::Archive {
                patch_id: patch_id.ok_or_else(|| anyhow!("a patch id must be provided"))?,
            },
            OperationName::Checkout => Operation::Checkout {
                patch_id: patch_id.ok_or_else(|| anyhow!("a patch must be provided"))?,
            },
            OperationName::Ready => Operation::Ready {
                patch_id: patch_id.ok_or_else(|| anyhow!("a patch must be provided"))?,
                undo,
            },
        };

        Ok((
            Options {
                op,
                fetch,
                push,
                verbose,
                announce,
            },
            vec![],
        ))
    }
}

pub fn run(options: Options, ctx: impl term::Context) -> anyhow::Result<()> {
    let (workdir, id) = radicle::rad::cwd()
        .map_err(|_| anyhow!("this command must be run in the context of a project"))?;

    let profile = ctx.profile()?;
    let repository = profile.storage.repository(id)?;

    transport::local::register(profile.storage.clone());

    if options.fetch {
        sync::fetch_all(repository.id(), &mut Node::new(profile.socket()))?;
    }

    match options.op {
        Operation::Open {
            ref message,
            draft,
            quiet,
        } => {
            create::run(
                &repository,
                &profile,
                &workdir,
                message.clone(),
                draft,
                quiet,
                options,
            )?;
        }
        Operation::List { filter } => {
            list::run(&repository, &profile, filter)?;
        }
        Operation::Show { patch_id, diff } => {
            let patch_id = patch_id.resolve(&repository.backend)?;
            show::run(&profile, &repository, &workdir, &patch_id, diff)?;
        }
        Operation::Update {
            ref patch_id,
            ref message,
            quiet,
        } => {
            let patch_id = patch_id
                .as_ref()
                .map(|id| id.resolve(&repository.backend))
                .transpose()?;
            update::run(
                &repository,
                &profile,
                &workdir,
                patch_id,
                message.clone(),
                quiet,
                &options,
            )?;
        }
        Operation::Archive { ref patch_id } => {
            let patch_id = patch_id.resolve::<PatchId>(&repository.backend)?;
            archive::run(&repository, &profile, &patch_id)?;
        }
        Operation::Ready { ref patch_id, undo } => {
            let patch_id = patch_id.resolve::<PatchId>(&repository.backend)?;
            ready::run(&repository, &profile, &patch_id, undo)?;
        }
        Operation::Delete { patch_id } => {
            let patch_id = patch_id.resolve(&repository.backend)?;
            delete::run(&repository, &profile, &patch_id)?;
        }
        Operation::Checkout { patch_id } => {
            let patch_id = patch_id.resolve(&repository.backend)?;
            checkout::run(&repository, &workdir, &patch_id)?;
        }
    }
    Ok(())
}
