mod cli;
mod cmd;
mod conventional;
mod error;
mod git;
mod semver;

use std::process::exit;

use conventional::config::make_cl_config;
use git::GitHelper;
use structopt::StructOpt;

pub(crate) use crate::{cmd::Command, error::Error};

fn main() -> Result<(), Error> {
    let opt: cli::Opt = cli::Opt::from_args();
    if let Some(path) = opt.path {
        std::env::set_current_dir(path)?;
    }
    let git = GitHelper::new("v")?;
    let config = make_cl_config(&git, opt.config.unwrap_or_else(|| ".versionrc".into()));
    let res = match opt.cmd {
        cli::Command::Check(cmd) => cmd.exec(config),
        cli::Command::Changelog(cmd) => cmd.exec(config),
        cli::Command::Version(cmd) => cmd.exec(config),
        cli::Command::Commit(cmd) => cmd.exec(config),
    };
    match res {
        Err(e) => {
            match e {
                Error::Check => (),
                _ => {
                    eprintln!("{}", e);
                }
            }
            exit(1)
        }
        _ => exit(0),
    }
}
