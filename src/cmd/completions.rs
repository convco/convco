#![cfg(feature = "completions")]
use clap::CommandFactory;
use clap_complete::aot::{generate, Shell};

use crate::{
    cli::{CompletionsCommand, Opt},
    cmd::Command,
    Config,
};

impl Command for CompletionsCommand {
    fn exec(&self, _: Config) -> anyhow::Result<()> {
        let mut cmd = Opt::command();
        let bin_name = cmd.get_name().to_string();
        let shell = self.shell.or_else(Shell::from_env).unwrap_or(Shell::Bash);
        generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        Ok(())
    }
}
