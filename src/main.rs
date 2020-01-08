#[macro_use]
extern crate lazy_static;

mod cli;
mod cmd;
mod conventional;
mod error;
mod git;

use crate::{cmd::Command, error::Error};

use std::process::exit;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
    let opt: cli::Opt = cli::Opt::from_args();
    if let Some(path) = opt.path {
        std::env::set_current_dir(path)?;
    }
    let res = match opt.cmd {
        cli::Command::Check(cmd) => cmd.exec(),
        cli::Command::Changelog(cmd) => cmd.exec(),
        cli::Command::Version(cmd) => cmd.exec(),
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
