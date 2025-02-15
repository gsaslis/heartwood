use std::ffi::OsString;
use std::path::Path;

use anyhow::Context as _;

use radicle::prelude::Id;
use radicle::rad;

use crate::terminal as term;
use crate::terminal::args;
use crate::terminal::args::{Args, Error, Help};

pub const HELP: Help = Help {
    name: "ls",
    description: "Create a fork of a project",
    version: env!("CARGO_PKG_VERSION"),
    usage: r#"
Usage

    rad fork [<rid>] [<option>...]

Options

    --help          Print help
"#,
};

pub struct Options {
    rid: Option<Id>,
}

impl Args for Options {
    fn from_args(args: Vec<OsString>) -> anyhow::Result<(Self, Vec<OsString>)> {
        use lexopt::prelude::*;

        let mut parser = lexopt::Parser::from_args(args);
        let mut rid = None;

        if let Some(arg) = parser.next()? {
            match arg {
                Long("help") => {
                    return Err(Error::Help.into());
                }
                Value(val) if rid.is_none() => {
                    rid = Some(args::rid(&val)?);
                }
                _ => return Err(anyhow::anyhow!(arg.unexpected())),
            }
        }

        Ok((Options { rid }, vec![]))
    }
}

pub fn run(options: Options, ctx: impl term::Context) -> anyhow::Result<()> {
    let profile = ctx.profile()?;
    let signer = profile.signer()?;
    let storage = &profile.storage;

    let rid = match options.rid {
        Some(rid) => rid,
        None => {
            let (_, rid) = radicle::rad::repo(Path::new("."))
                .context("Current directory is not a radicle project")?;

            rid
        }
    };

    rad::fork(rid, &signer, &storage)?;
    term::success!("Forked project {rid} for {}", profile.id());

    Ok(())
}
