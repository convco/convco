use std::{path::PathBuf, str::FromStr};

use clap::Parser;

#[derive(Debug, Parser)]
#[clap(name = "convco", about = "Conventional commit tools", version)]
pub struct Opt {
    /// Run as if convco was started in <path> instead of the current working directory.
    #[clap(short = 'C', global = true)]
    pub path: Option<PathBuf>,
    #[clap(short = 'c', long = "config", global = true)]
    pub config: Option<PathBuf>,
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Parser)]
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

#[derive(Debug, Parser)]
pub struct VersionCommand {
    /// Prefix used in front of the semantic version
    #[clap(short, long, default_value = "v")]
    pub prefix: String,
    /// Revision to show the version for
    #[clap(default_value = "HEAD")]
    pub rev: String,
    /// Get the next version
    #[clap(short, long)]
    pub bump: bool,
    /// Instead of printing out the bumped version, prints out one of: major, minor or patch
    #[clap(short, long, conflicts_with_all(&["major", "minor", "patch"]))]
    pub label: bool,
    /// Bump to a major release version, regardless of the conventional commits
    #[clap(long)]
    pub major: bool,
    /// Bump to a minor release version, regardless of the conventional commits
    #[clap(long)]
    pub minor: bool,
    /// Bump to a patch release version, regardless of the conventional commits
    #[clap(long)]
    pub patch: bool,
    /// Only commits that update those <paths> will be taken into account. It is useful to support monorepos.
    #[clap(short = 'P', long)]
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct CheckCommand {
    /// Start of the revwalk, can also be a commit range. Can be in the form `<commit>..<commit>`.
    #[clap(default_value = "HEAD")]
    pub rev: String,
    /// Limit the number of commits to check.
    #[clap(short, long = "max-count")]
    pub number: Option<usize>,
    /// Include conventional merge commits (commits with more than 1 parent) in the changelog.
    #[clap(long)]
    pub merges: bool,
    /// Follow only the first parent
    #[clap(long)]
    pub first_parent: bool,
    /// Ignore commits created by `git revert` commands
    #[clap(long)]
    pub ignore_reverts: bool,
}

#[derive(Debug, Parser)]
pub struct ChangelogCommand {
    /// Prefix used in front of the semantic version.
    #[clap(short, long, default_value = "v")]
    pub prefix: String,
    #[clap(default_value = "HEAD")]
    pub rev: String,
    #[clap(short, long)]
    pub skip_empty: bool,
    /// Limits the number of version tags to add in the changelog.
    #[clap(short, long)]
    pub max_versions: Option<usize>,
    /// Only print this number of major versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true)]
    pub max_minors: u64,
    /// Only show this number of minor versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true)]
    pub max_majors: u64,
    /// Only show this number of patch versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true)]
    pub max_patches: u64,
    /// Do not generate links. Overrides linkReferences and linkCompare in the config.
    #[clap(short, long)]
    pub no_links: bool,
    /// Include conventional merge commits (commits with more than 1 parent) in the changelog.
    #[clap(long)]
    pub merges: bool,
    /// Print hidden sections
    #[clap(long)]
    pub include_hidden_sections: bool,
    /// Only commits that update those <paths> will be taken into account. It is useful to support monorepos.
    #[clap(short = 'P', long)]
    pub paths: Vec<PathBuf>,
    /// Follow only the first parent of merge commits. Commits from the merged branche(s) will be discarded.
    #[clap(long)]
    pub first_parent: bool,
}

#[derive(Debug, Parser)]
pub struct CommitCommand {
    /// A bug fix
    #[clap(long,
        conflicts_with_all(&["feat", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test", "type"]),
    )]
    pub fix: bool,
    /// A new feature
    #[clap(long,
        conflicts_with_all(&["fix", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test", "type"]),
    )]
    pub feat: bool,
    /// Changes that affect the build system or external dependencies
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "chore", "ci", "docs", "style", "refactor", "perf", "test", "type"]),
    )]
    pub build: bool,
    /// Other changes that don't modify src or test files
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "ci", "docs", "style", "refactor", "perf", "test", "type"]),
    )]
    pub chore: bool,
    /// Changes to CI configuration files and scripts
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "docs", "style", "refactor", "perf", "test", "type"]),
    )]
    pub ci: bool,
    /// Documentation only changes
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "style", "refactor", "perf", "test", "type"]),
    )]
    pub docs: bool,
    /// Changes that do not affect the meaning of the code (e.g. formatting)
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "refactor", "perf", "test", "type"]),
    )]
    pub style: bool,
    /// A code change that neither fixes a bug nor adds a feature
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "perf", "test", "type"]),
    )]
    pub refactor: bool,
    /// A code change that improves performance
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "test", "type"]),
    )]
    pub perf: bool,
    /// Adding missing tests or correcting existing tests
    #[clap(long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "perf", "type"]),
    )]
    pub test: bool,
    /// Specify your own commit type
    #[clap(short, long,
        conflicts_with_all(&["feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test"]),
    )]
    pub r#type: Option<String>,
    /// Specifies the scope of the message
    #[clap(short, long)]
    pub scope: Option<String>,
    /// The first message will be the description. Other -m options will be used as the body.
    #[clap(short, long)]
    pub message: Vec<String>,
    /// Specify extra footers to the message
    #[clap(
        short,
        long,
        visible_alias = "trailer",
        value_name = "token>(=|:)<value"
    )]
    pub footers: Vec<Footer>,
    /// Introduces a breaking change
    #[clap(long)]
    pub breaking: bool,
    /// Interactive mode
    #[clap(long, short)]
    pub interactive: bool,
    /// Extra arguments passed to the git commit command
    #[clap(last = true)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Footer(pub(crate) String, pub(crate) String);

impl From<(&str, &str)> for Footer {
    fn from(s: (&str, &str)) -> Self {
        Self(s.0.trim().to_owned(), s.1.trim().to_owned())
    }
}

impl FromStr for Footer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        match (split.next(), split.next()) {
            (Some(k), Some(v)) => Ok((k, v).into()),
            _ => {
                let mut split = s.split('=');
                match (split.next(), split.next()) {
                    (Some(k), Some(v)) => Ok((k, v).into()),
                    _ => Err(format!("invalid footer: {}", s)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footer() {
        let footer: Footer = "Reviewed-by: Z".parse().unwrap();
        assert_eq!(Footer("Reviewed-by".into(), "Z".into()), footer);
    }

    #[test]
    fn test_footer2() {
        let footer: Footer = "Reviewed-by=Z".parse().unwrap();
        assert_eq!(Footer("Reviewed-by".into(), "Z".into()), footer);
    }

    #[test]
    fn test_footer_err_empty() {
        let err: String = "".parse::<Footer>().unwrap_err();
        assert_eq!(err, format!("invalid footer: {}", ""));
    }
}
