use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "convco", about = "Conventional commit tools")]
pub struct Opt {
    /// Run as if convco was started in <path> instead of the current working directory.
    #[structopt(short = "C", global = true)]
    pub path: Option<PathBuf>,
    #[structopt(short = "c", long = "config", global = true)]
    pub config: Option<PathBuf>,
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
    /// Helps to make conventional commits.
    Commit(CommitCommand),
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
    /// Start of the revwalk, can also be a commit range. Can be in the form `<commit>..<commit>`.
    #[structopt(default_value = "HEAD")]
    pub rev: String,
    /// Limit the number of commits to check.
    #[structopt(short, long = "max-count")]
    pub number: Option<usize>,
}

#[derive(Debug, StructOpt)]
pub struct ChangelogCommand {
    /// Prefix used in front of the semantic version.
    #[structopt(short, long, default_value = "v")]
    pub prefix: String,
    #[structopt(default_value = "HEAD")]
    pub rev: String,
    #[structopt(short, long)]
    pub skip_empty: bool,
}

#[derive(Debug, StructOpt)]
pub struct CommitCommand {
    /// A bug fix
    #[structopt(long,
        conflicts_with_all(&["feat", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub fix: bool,
    /// A new feature
    #[structopt(long,
        conflicts_with_all(&["fix", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub feat: bool,
    /// Changes that affect the build system or external dependencies
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "chore", "ci", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub build: bool,
    /// Other changes that don't modify src or test files
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "ci", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub chore: bool,
    /// Changes to CI configuration files and scripts
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub ci: bool,
    /// Documentation only changes
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "style", "refactor", "perf", "test"]),
    )]
    pub docs: bool,
    /// Changes that do not affect the meaning of the code (e.g. formatting)
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "refactor", "perf", "test"]),
    )]
    pub style: bool,
    /// A code change that neither fixes a bug nor adds a feature
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "perf", "test"]),
    )]
    pub refactor: bool,
    /// A code change that improves performance
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "test"]),
    )]
    pub perf: bool,
    /// Adding missing tests or correcting existing tests
    #[structopt(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "perf"]),
    )]
    pub test: bool,
    /// Introduces a breaking change
    #[structopt(long)]
    pub breaking: bool,
    /// Extra arguments passed to the git command
    #[structopt(last = true)]
    pub extra_args: Vec<String>,
}
