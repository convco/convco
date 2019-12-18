use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "convco", about = "Conventional commit tools")]
pub struct Opt {
    /// Run as if git was started in <path> instead of the current working directory.
    #[structopt(short = "C", global = true)]
    pub path: Option<PathBuf>,
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Verifies if all commits are conventional
    Check(CheckCommand),
    /// Writes out a changelog
    Changelog(ChangelogCommand),
    /// Show the current version
    Version(VersionCommand),
}

#[derive(Debug, StructOpt)]
pub struct VersionCommand {
    /// Prefix used in front of the semantic version
    #[structopt(short, long, default_value = "v")]
    pub prefix: String,
    /// Revision to show the version for
    #[structopt(default_value = "HEAD")]
    pub rev: String,
    /// Get the next version
    #[structopt(short, long)]
    pub bump: bool,
    /// Instead of printing out the bumped version, prints out one of: major, minor or patch
    #[structopt(short, long, conflicts_with_all(&["major", "minor", "patch"]))]
    pub label: bool,
    /// Bump to a major release version, regardless of the conventional commits
    #[structopt(long)]
    pub major: bool,
    /// Bump to a minor release version, regardless of the conventional commits
    #[structopt(long)]
    pub minor: bool,
    /// Bump to a patch release version, regardless of the conventional commits
    #[structopt(long)]
    pub patch: bool,
}

#[derive(Debug, StructOpt)]
pub struct CheckCommand {
    #[structopt(default_value = "HEAD")]
    pub rev: String,
}

#[derive(Debug, StructOpt)]
pub struct ChangelogCommand {
    /// Prefix used in front of the semantic version.
    #[structopt(short, long, default_value = "v")]
    pub prefix: String,
    #[structopt(default_value = "HEAD")]
    pub rev: String,
}
