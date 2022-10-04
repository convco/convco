mod cli;
mod cmd;
mod conventional;
mod error;
mod git;
mod semver;

use std::process::exit;

use clap::Parser;
use conventional::config::make_cl_config;
use git::GitHelper;

pub(crate) use crate::{cmd::Command, error::Error};

fn main() -> anyhow::Result<()> {
    let opt: cli::Opt = cli::Opt::parse();
    if let Some(path) = opt.path {
        std::env::set_current_dir(path)?;
    }
    let git = GitHelper::new("v").map_err(|e|
        if e.message().contains("config value 'safe.directory' was not found") {
            anyhow::Error::new(e).context("Could not open the git repository.\nIf run from docker set the right user id and group id.\nE.g. `docker run -u \"$(id -u):$(id -g)\" -v \"$PWD:/tmp\" --workdir /tmp --rm convco/convco`")
        } else {
            anyhow::Error::new(e)
        }
        )?;
    let config = make_cl_config(&git, opt.config.unwrap_or_else(|| ".versionrc".into()));
    let res = match opt.cmd {
        cli::Command::Check(cmd) => cmd.exec(config),
        cli::Command::Changelog(cmd) => cmd.exec(config),
        cli::Command::Version(cmd) => cmd.exec(config),
        cli::Command::Commit(cmd) => cmd.exec(config),
    };
    match res {
        Err(e) => {
            match e.downcast_ref::<Error>() {
                Some(Error::Check) => (),
                _ => {
                    eprintln!("{:?}", e);
                }
            }
            exit(1)
        }
        _ => exit(0),
    }
}
