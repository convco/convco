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

    let repo = open_repo()?;
    let config_path = config.unwrap_or_else(|| match PathBuf::from(".convco") {
        p if p.is_file() => p,
        _ => ".versionrc".into(),
    });
    let config = Config::from_repo(&repo, config_path)?;

    let res = match cmd {
        cli::Command::Config(command) => command.exec(config),
        cli::Command::Check(command) => command.exec(config),
        cli::Command::Changelog(command) => command.exec(config),
        cli::Command::Version(command) => command.exec(config),
        cli::Command::Commit(command) => command.exec(config),
        #[cfg(feature = "completions")]
        cli::Command::Completions(cmd) => cmd.exec(config),
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
