use std::{
    fmt,
    path::{Path, PathBuf},
};

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

use crate::{error::Error, git::GitHelper};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Increment {
    Major,
    Minor,
    Patch,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Type {
    pub(crate) r#type: String,
    pub(crate) increment: Increment,
    #[serde(default)]
    pub(crate) section: String,
    #[serde(default)]
    pub(crate) hidden: bool,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.r#type)
    }
}

/// see: [Conventional Changelog Configuration](https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md)
/// Additional config: `host`, `owner`, `repository`, `scope_regex` and `template`
/// Those values are derived from `git remote origin get-url` if not set.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    /// A string to be used as the main header section of the CHANGELOG.
    #[serde(default = "default_header")]
    pub(crate) header: String,
    /// An array of `type` objects representing the explicitly supported commit message types, and whether they should show up in generated `CHANGELOG`s.
    #[serde(default = "default_types")]
    #[serde(deserialize_with = "deserialize_type")]
    pub(crate) types: Vec<Type>,
    /// Boolean indicating whether or not the action being run (generating CHANGELOG, recommendedBump, etc.) is being performed for a pre-major release (<1.0.0).\n This config setting will generally be set by tooling and not a user.
    #[serde(default)]
    pub(crate) pre_major: bool,
    /// A URL representing a specific commit at a hash.
    #[serde(default = "default_commit_url_format")]
    pub(crate) commit_url_format: String,
    /// A URL representing the comparison between two git SHAs.
    #[serde(default = "default_compare_url_format")]
    pub(crate) compare_url_format: String,
    /// A URL representing the issue format (allowing a different URL format to be swapped in for Gitlab, Bitbucket, etc).
    #[serde(default = "default_issue_url_format")]
    pub(crate) issue_url_format: String,
    /// A URL representing the a user's profile URL on GitHub, Gitlab, etc. This URL is used for substituting @bcoe with https://github.com/bcoe in commit messages.
    #[serde(default = "default_user_url_format")]
    pub(crate) user_url_format: String,
    /// A string to be used to format the auto-generated release commit message.
    #[serde(default = "default_release_commit_message_format")]
    pub(crate) release_commit_message_format: String,
    /// An array of prefixes used to detect references to issues
    #[serde(default = "default_issue_prefixes")]
    pub(crate) issue_prefixes: Vec<String>,

    pub(crate) host: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) repository: Option<String>,
    /// `template`. An optional template directory. The template should be called `template.hbs`. Partials can be used.
    pub(crate) template: Option<PathBuf>,
    /// `commitTemplate`. An optional template file for convco commit.
    pub(crate) commit_template: Option<PathBuf>,
    /// `scopeRegex`. A regex to define possible scopes.
    /// For this project this could be `"changelog|check|commit|version"`.
    /// Defaults to `"^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$"`.
    #[serde(default = "default_scope_regex")]
    pub(crate) scope_regex: String,
    /// Default number of characters in a single line of the CHANGELOG.
    /// This only makes sense if the template makes use of `{{#word-wrap}}` blocks.
    #[serde(default = "default_line_length")]
    pub(crate) line_length: usize,
    /// Disable word-wrap in the CHANGELOG.
    /// This only makes sense if the template makes use of `{{#word-wrap}}` blocks.
    #[serde(default)]
    pub(crate) wrap_disabled: bool,
    /// Add link to compare 2 versions.
    #[serde(default = "default_true")]
    pub(crate) link_compare: bool,
    /// Link commit and issue references in the changelog.
    #[serde(default = "default_true")]
    pub(crate) link_references: bool,
    /// Include merge commits
    #[serde(default)]
    pub(crate) merges: bool,
    /// Follow only the first parent
    #[serde(default)]
    pub first_parent: bool,
    /// Strip the commit message(s) by the given regex pattern
    #[serde(default = "default_strip_regex")]
    pub(crate) strip_regex: String,
    #[serde(default)]
    pub(crate) description: DescriptionConfig,
    /// Initial version to use if no previous version is found
    #[serde(default = "default_initial_bump_version")]
    pub(crate) initial_bump_version: Version,
    /// Ignore commits that update only those paths when calculating bumped versions.
    /// Each path should be relative to the root of the repository.
    #[serde(default, alias = "ignore_paths", alias = "ignore-paths")]
    pub(crate) ignore_paths: Vec<PathBuf>,
}

fn default_initial_bump_version() -> Version {
    Version::new(0, 1, 0)
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub(crate) struct DescriptionConfig {
    pub(crate) length: DescriptionLengthConfig,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DescriptionLengthConfig {
    /// Define the minimum length of the description when using convco commit
    #[serde(default = "default_some_10")]
    pub(crate) min: Option<usize>,
    /// Define the maximum length of the description when using convco commit
    pub(crate) max: Option<usize>,
}

impl Default for DescriptionLengthConfig {
    fn default() -> Self {
        Self {
            min: Some(10),
            max: None,
        }
    }
}

fn deserialize_type<'de, D>(deserializer: D) -> Result<Vec<Type>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct PartialType {
        r#type: String,
        increment: Option<Increment>,
        section: String,
        #[serde(default)]
        hidden: bool,
    }

    let vec: Result<Vec<PartialType>, D::Error> = Deserialize::deserialize(deserializer);
    vec.map(|vec| {
        vec.into_iter()
            .map(
                |PartialType {
                     r#type,
                     increment,
                     section,
                     hidden,
                 }| Type {
                    r#type: r#type.clone(),
                    increment: increment.unwrap_or(match r#type.as_str() {
                        "feat" => Increment::Minor,
                        "fix" => Increment::Patch,
                        _ => Increment::None,
                    }),
                    section,
                    hidden,
                },
            )
            .collect()
    })
}

const fn default_true() -> bool {
    true
}

const fn default_some_10() -> Option<usize> {
    Some(10)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            header: default_header(),
            types: default_types(),
            pre_major: false,
            commit_url_format: default_commit_url_format(),
            compare_url_format: default_compare_url_format(),
            issue_url_format: default_issue_url_format(),
            user_url_format: default_user_url_format(),
            release_commit_message_format: default_release_commit_message_format(),
            issue_prefixes: default_issue_prefixes(),
            line_length: default_line_length(),
            host: None,
            owner: None,
            repository: None,
            template: None,
            commit_template: None,
            scope_regex: "^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$".to_string(),
            link_compare: true,
            link_references: true,
            merges: false,
            first_parent: false,
            wrap_disabled: false,
            strip_regex: "".to_string(),
            description: Default::default(),
            initial_bump_version: Version::new(0, 1, 0),
            ignore_paths: Vec::new(),
        }
    }
}

fn default_header() -> String {
    "# Changelog\n".into()
}

fn default_types() -> Vec<Type> {
    vec![
        Type {
            r#type: "feat".into(),
            increment: Increment::Minor,
            section: "Features".into(),
            hidden: false,
        },
        Type {
            r#type: "fix".into(),
            increment: Increment::Patch,
            section: "Fixes".into(),
            hidden: false,
        },
        Type {
            r#type: "build".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "chore".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "ci".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "docs".into(),
            increment: Increment::None,
            section: "Documentation".into(),
            hidden: true,
        },
        Type {
            r#type: "style".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "refactor".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "perf".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
        Type {
            r#type: "test".into(),
            increment: Increment::None,
            section: "Other".into(),
            hidden: true,
        },
    ]
}

fn default_commit_url_format() -> String {
    "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/commit/{{hash}}".into()
}

fn default_compare_url_format() -> String {
    "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/compare/{{previousTag}}...{{currentTag}}"
        .into()
}

fn default_issue_url_format() -> String {
    "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/issues/{{issue}}".into()
}

fn default_user_url_format() -> String {
    "{{host}}/{{user}}".into()
}

fn default_release_commit_message_format() -> String {
    "chore(release): {{currentTag}}".into()
}
fn default_line_length() -> usize {
    80
}

fn default_issue_prefixes() -> Vec<String> {
    vec!["#".into()]
}

fn default_scope_regex() -> String {
    "^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$".to_string()
}

fn default_strip_regex() -> String {
    "".to_string()
}

type HostOwnerRepo = (Option<String>, Option<String>, Option<String>);

/// Get host, owner and repository based on the git remote origin url.
pub(crate) fn host_info(git: &GitHelper) -> Result<HostOwnerRepo, Error> {
    if let Some(mut url) = git.url()? {
        if !url.contains("://") {
            // check if it contains a port
            if let Some(colon) = url.find(':') {
                match url.as_bytes()[colon + 1] {
                    b'0'..=b'9' => url = format!("scheme://{}", url),
                    _ => url = format!("scheme://{}/{}", &url[..colon], &url[colon + 1..]),
                }
            }
        }
        let url = Url::parse(url.as_str())?;
        host_info_from_url(url)
    } else {
        Ok((None, None, None))
    }
}

fn host_info_from_url(url: Url) -> Result<HostOwnerRepo, Error> {
    let scheme = match url.scheme() {
        "scheme" => "https",
        scheme => scheme,
    };
    let host = url.host().map(|h| format!("{scheme}://{}", h));
    let (owner, repository) = match url.path().rsplit_once('/') {
        Some((owner, repository)) => {
            let owner = Some(owner.trim_start_matches('/').to_owned());
            let repository = Some(repository.trim_end_matches(".git").to_owned());
            (owner, repository)
        }
        None => (None, None),
    };
    Ok((host, owner, repository))
}

pub(crate) fn make_cl_config(git: Option<GitHelper>, path: impl AsRef<Path>) -> Config {
    let mut config: Config = std::fs::read(path)
        .ok()
        .and_then(|vec| (serde_norway::from_reader(vec.as_slice())).ok())
        .unwrap_or_default();
    if let Config {
        host: None,
        owner: None,
        repository: None,
        ..
    } = config
    {
        if let Some(ref git) = git {
            if let Ok((host, owner, repository)) = host_info(git) {
                config.host = host;
                config.owner = owner;
                config.repository = repository;
            }
        }
    }

    if config.host.is_none() || config.commit_url_format.is_empty() {
        config.link_references = false;
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_info_from_url() {
        fn assert_all(url: &str, host: &str, owner: &str, repo: &str) {
            let expected: HostOwnerRepo = (
                Some(host.to_string()),
                Some(owner.to_string()),
                Some(repo.to_string()),
            );
            let result = host_info_from_url(url.parse().unwrap()).unwrap();
            assert_eq!(result, expected);
        }
        assert_all(
            "https://github.com/convco/convco.git",
            "https://github.com",
            "convco",
            "convco",
        );
        assert_all(
            "http://github.com/convco/convco.git",
            "http://github.com",
            "convco",
            "convco",
        );
        assert_all(
            "https://gitlab.com/group/subgroup/repo.git",
            "https://gitlab.com",
            "group/subgroup",
            "repo",
        );
        assert_all(
            // git@github.com:convco/convco.git is replaced to scheme://git@github.com/convco/convco.git
            "scheme://git@github.com/convco/convco.git",
            "https://github.com",
            "convco",
            "convco",
        );
    }

    #[test]
    fn test() {
        let json = r#"{
              "types": [
                {"type": "chore", "section":"Others", "hidden": false},
                {"type": "revert", "section":"Reverts", "hidden": false},
                {"type": "feat", "section": "Features", "hidden": false},
                {"type": "fix", "section": "Bug Fixes", "hidden": false},
                {"type": "improvement", "section": "Feature Improvements", "hidden": false},
                {"type": "docs", "section":"Docs", "hidden": false},
                {"type": "style", "section":"Styling", "hidden": false},
                {"type": "refactor", "section":"Code Refactoring", "hidden": false},
                {"type": "perf", "section":"Performance Improvements", "hidden": false},
                {"type": "test", "section":"Tests", "hidden": false},
                {"type": "build", "section":"Build System", "hidden": false},
                {"type": "ci", "section":"CI", "hidden":false}
              ],
            }"#;
        let value: Config = serde_norway::from_str(json).unwrap();
        assert_eq!(
            value,
            Config {
                line_length: 80,
                header: "# Changelog\n".to_string(),
                types: vec![
                    Type {
                        r#type: "chore".into(),
                        increment: Increment::None,
                        section: "Others".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "revert".into(),
                        increment: Increment::None,
                        section: "Reverts".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "feat".into(),
                        increment: Increment::Minor,
                        section: "Features".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "fix".into(),
                        increment: Increment::Patch,
                        section: "Bug Fixes".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "improvement".into(),
                        increment: Increment::None,
                        section: "Feature Improvements".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "docs".into(),
                        increment: Increment::None,
                        section: "Docs".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "style".into(),
                        increment: Increment::None,
                        section: "Styling".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "refactor".into(),
                        increment: Increment::None,
                        section: "Code Refactoring".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "perf".into(),
                        increment: Increment::None,
                        section: "Performance Improvements".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "test".into(),
                        increment: Increment::None,
                        section: "Tests".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "build".into(),
                        increment: Increment::None,
                        section: "Build System".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "ci".into(),
                        increment: Increment::None,
                        section: "CI".into(),
                        hidden: false
                    }
                ],
                pre_major: false,
                commit_url_format: "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/commit/{{hash}}"
                    .to_string(),
                compare_url_format:
                    "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/compare/{{previousTag}}...{{currentTag}}"
                        .to_string(),
                issue_url_format:
                    "{{@root.host}}/{{@root.owner}}/{{@root.repository}}/issues/{{issue}}"
                        .to_string(),
                user_url_format: "{{host}}/{{user}}".to_string(),
                release_commit_message_format: "chore(release): {{currentTag}}".to_string(),
                issue_prefixes: vec!["#".into()],
                host: None,
                owner: None,
                repository: None,
                template: None,
                commit_template: None,
                scope_regex: "^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$".to_string(),
                link_compare: true,
                link_references: true,
                merges: false,
                first_parent: false,
                wrap_disabled: false,
                strip_regex: "".to_string(),
                description: DescriptionConfig { length: DescriptionLengthConfig { min: Some(10), max: None } },
                initial_bump_version: Version::new(0, 1, 0),
                ignore_paths: Vec::new(),
            }
        )
    }
}
