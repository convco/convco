use std::{path::PathBuf, str::FromStr};

use clap::Parser;
#[cfg(feature = "completions")]
use clap_complete::aot::Shell as Shells;
use semver::{Prerelease, Version};

#[derive(Debug, Parser)]
#[clap(name = "convco", about = "Conventional commit tools", version)]
pub struct Opt {
    /// Run as if convco was started in <path> instead of the current working directory.
    #[clap(short = 'C', global = true)]
    pub path: Option<PathBuf>,
    #[clap(short = 'c', long = "config", global = true, env = "CONVCO_CONFIG")]
    pub config: Option<PathBuf>,
    #[clap(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    /// Actions for configuration
    Config(ConfigCommand),
    /// Verifies if all commits are conventional
    Check(CheckCommand),
    #[cfg(feature = "completions")]
    /// Generates shell completions
    Completions(CompletionsCommand),
    /// Writes out a changelog
    Changelog(ChangelogCommand),
    /// Show the current version
    Version(VersionCommand),
    /// Helps to make conventional commits.
    Commit(CommitCommand),
}

#[derive(Debug, Parser)]
pub struct ConfigCommand {
    /// Print out the default configuration instead of the current configuration.
    #[clap(short, long)]
    pub default: bool,
}

#[derive(Debug, Parser)]
pub struct VersionCommand {
    /// Prefix used in front of the semantic version
    #[clap(short, long, default_value = "v", env = "CONVCO_PREFIX")]
    pub prefix: String,
    /// Print prefix in front of the semantic version
    #[clap(long, visible_alias = "pp", env = "CONVCO_PRINT_PREFIX")]
    pub print_prefix: bool,
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
    #[clap(long, env = "CONVCO_FORCE_MAJOR_BUMP")]
    pub major: bool,
    /// Bump to a minor release version, regardless of the conventional commits
    #[clap(long, env = "CONVCO_FORCE_MINOR_BUMP")]
    pub minor: bool,
    /// Bump to a patch release version, regardless of the conventional commits
    #[clap(long, env = "CONVCO_FORCE_PATCH_BUMP")]
    pub patch: bool,
    /// Suffix with a prerelease version. Requires --bump.
    #[clap(long, requires = "bump", default_value_t = Prerelease::new("").unwrap())]
    pub prerelease: Prerelease,
    /// Only commits that update those <paths> will be taken into account. It is useful to support monorepos.
    /// Each path should be relative to the root of the repository.
    #[clap(short = 'P', long, env = "CONVCO_PATHS")]
    pub paths: Vec<PathBuf>,
    /// Ignore commits that update only those <paths> when calculating the bumped version.
    /// Each path should be relative to the root of the repository.
    #[clap(long = "ignore-path", visible_alias = "ignore-paths", env = "CONVCO_IGNORE_PATHS")]
    pub ignore_paths: Vec<PathBuf>,
    /// Print the commit-sha of the version instead of the semantic version
    #[clap(long)]
    pub commit_sha: bool,
    /// Ignore pre-release versions when finding the last version
    #[clap(long, env = "CONVCO_IGNORE_PRERELEASES")]
    pub ignore_prereleases: bool,
    /// If no version is found use this version for the first bump
    #[clap(long, env = "CONVCO_INITIAL_BUMP_VERSION")]
    pub initial_bump_version: Option<Version>,
}

#[derive(Debug, Parser)]
pub struct CheckCommand {
    /// Start of the revwalk, can also be a commit range. Can be in the form `<commit>..<commit>`.
    #[clap(env = "CONVCO_REV")]
    pub rev: Option<String>,
    /// Limit the number of commits to check.
    #[clap(short, long = "max-count", env = "CONVCO_MAX_COUNT")]
    pub number: Option<usize>,
    /// Include conventional merge commits (commits with more than 1 parent) in the changelog.
    #[clap(long, env = "CONVCO_MERGES")]
    pub merges: bool,
    /// Follow only the first parent
    #[clap(long, env = "CONVCO_FIRST_PARENT")]
    pub first_parent: bool,
    /// Ignore commits created by `git revert` commands
    #[clap(long, env = "CONVCO_IGNORE_REVERTS")]
    pub ignore_reverts: bool,
    /// Read a single commit message from stdin
    #[clap(long)]
    pub from_stdin: bool,
    /// String comments and whitespace from commit message
    /// This is similar to `git commit --cleanup=strip`
    #[clap(long, requires("from_stdin"))]
    pub strip: bool,
}

#[cfg(feature = "completions")]
#[derive(Debug, Parser)]
pub struct CompletionsCommand {
    /// Shell to generate completions for
    pub shell: Option<Shells>,
}

#[derive(Debug, Parser)]
pub struct ChangelogCommand {
    /// Prefix used in front of the semantic version.
    #[clap(short, long, default_value = "v", env = "CONVCO_PREFIX")]
    pub prefix: String,
    #[clap(default_value = "HEAD", env = "CONVCO_REV")]
    pub rev: String,
    #[clap(short, long, env = "CONVCO_SKIP_EMPTY")]
    pub skip_empty: bool,
    /// Limits the number of version tags to add in the changelog.
    #[clap(short, long, env = "CONVCO_MAX_VERSIONS")]
    pub max_versions: Option<usize>,
    /// Only print this number of minor versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true, env = "CONVCO_MAX_MINORS")]
    pub max_minors: u64,
    /// Only show this number of major versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true, env = "CONVCO_MAX_MAJORS")]
    pub max_majors: u64,
    /// Only show this number of patch versions.
    #[clap(long, default_value_t=u64::MAX, hide_default_value=true, env = "CONVCO_MAX_PATCHES")]
    pub max_patches: u64,
    /// Ignore pre-release versions when finding the last version
    #[clap(long, env = "CONVCO_IGNORE_PRERELEASES")]
    pub ignore_prereleases: bool,
    /// Do not generate links. Overrides linkReferences and linkCompare in the config.
    #[clap(short, long, env = "CONVCO_NO_LINKS")]
    pub no_links: bool,
    /// Include conventional merge commits (commits with more than 1 parent) in the changelog.
    #[clap(long, env = "CONVCO_MERGES")]
    pub merges: bool,
    /// Print hidden sections
    #[clap(long, env = "CONVCO_INCLUDE_HIDDEN_SECTIONS")]
    pub include_hidden_sections: bool,
    /// Only commits that update those <paths> will be taken into account. It is useful to support monorepos.
    /// Each path should be relative to the root of the repository.
    #[clap(short = 'P', long, env = "CONVCO_PATHS")]
    pub paths: Vec<PathBuf>,
    /// Ignore commits that update only those <paths>.
    /// Each path should be relative to the root of the repository.
    #[clap(long = "ignore-path", visible_alias = "ignore-paths", env = "CONVCO_IGNORE_PATHS")]
    pub ignore_paths: Vec<PathBuf>,
    /// Follow only the first parent of merge commits. Commits from the merged branche(s) will be discarded.
    #[clap(long, env = "CONVCO_FIRST_PARENT")]
    pub first_parent: bool,
    /// Max line length before wrapping.
    /// This only makes sense if the template makes use of `{{#word-wrap}}` blocks.
    #[clap(long, env = "CONVCO_LINE_LENGTH")]
    pub line_length: Option<usize>,
    /// Do not wrap lines.
    /// This only makes sense if the template makes use of `{{#word-wrap}}` blocks.
    #[clap(long, env = "CONVCO_NO_WRAP")]
    pub no_wrap: bool,
    /// Change the title for the unreleased commits.
    /// If a semantic version is given, the title will be prefixed.
    #[clap(short, long, default_value = "Unreleased", env = "CONVCO_UNRELEASED")]
    pub unreleased: String,
    /// Path to write the changelog to.
    #[clap(short, long, default_value = "-", env = "CONVCO_OUTPUT")]
    pub output: PathBuf,
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
    /// Interactive mode. Start the wizard if no type and description is given.
    #[clap(long, short, env = "CONVCO_INTERACTIVE")]
    pub interactive: bool,
    /// Runs `git add -N <PATH>`.
    /// An entry for the path is placed in the index with no content.
    /// This is useful in combination with --patch.
    #[clap(short = 'N', long, env = "CONVCO_INTENT_TO_ADD")]
    pub intent_to_add: Vec<PathBuf>,
    /// Runs `git add -p`.
    /// Interactively choose hunks of patch between the index and the work tree.
    #[clap(short, long, env = "CONVCO_PATCH")]
    pub patch: bool,
    /// Path to store the commit message to recover from in case of an error
    /// If the path is `$GIT_DIR/COMMIT_EDITMSG` convco will not call `git commit`
    #[clap(hide = true)]
    pub commit_msg_path: Option<PathBuf>,
    /// Extra arguments passed to the git commit command
    #[clap(last = true)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
