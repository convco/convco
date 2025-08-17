use std::{path::PathBuf, process::exit};

use clap::Parser;
use cmd::Command;
use convco::{open_repo, Config, ConvcoError};
mod cli;
mod cmd;

fn main() -> anyhow::Result<()> {
    let cli::Opt {
        path, config, cmd, ..
    } = cli::Opt::parse();
    // cli::Opt::parse_from(["convco", "-C", "../convco", "changelog"]);

    if let Some(path) = path {
        std::env::set_current_dir(path)?;
    }

    let config_path = config.unwrap_or_else(|| match PathBuf::from(".convco") {
        p if p.is_file() => p,
        _ => ".versionrc".into(),
    });

    let res = match cmd {
        cli::Command::Config(command) => {
            let config = if command.default {
                Config::default()
            } else {
                match open_repo() {
                    Ok(repo) => Config::from_repo(&repo, &config_path)?,
                    Err(_) => Config::from_path(&config_path),
                }
            };
            command.exec(config)
        }
        cli::Command::Check(command) => {
            let repo = open_repo()?;
            command.exec(Config::from_repo(&repo, config_path)?)
        }
        cli::Command::Changelog(command) => {
            let repo = open_repo()?;
            command.exec(Config::from_repo(&repo, config_path)?)
        }
        cli::Command::Version(command) => {
            let repo = open_repo()?;
            command.exec(Config::from_repo(&repo, config_path)?)
        }
        cli::Command::Commit(command) => {
            let repo = open_repo()?;
            command.exec(Config::from_repo(&repo, config_path)?)
        }
        #[cfg(feature = "completions")]
        cli::Command::Completions(command) => command.exec(Config::default()),
    };
    match res {
        Err(e) => {
            match e.downcast_ref::<ConvcoError>() {
                Some(ConvcoError::Check) => (),
                _ => {
                    eprintln!("{:?}", e);
                }
            }
            exit(1)
        }
        _ => exit(0),
    }
}
