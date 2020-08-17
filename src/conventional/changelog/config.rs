use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Type {
    pub(crate) r#type: String,
    #[serde(default)]
    pub(crate) section: String,
    #[serde(default)]
    pub(crate) hidden: bool,
}

/// see: [Conventional Changelog Configuration](https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md)
/// Additional config: `host`, `owner`, `repository` and `template`
/// Those values are derived from `git remote origin get-url` if not set.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    /// A string to be used as the main header section of the CHANGELOG.
    #[serde(default = "default_header")]
    pub(crate) header: String,
    /// An array of `type` objects representing the explicitly supported commit message types, and whether they should show up in generated `CHANGELOG`s.
    #[serde(default = "default_types")]
    pub(crate) types: Vec<Type>,
    /// Boolean indicating whether or not the action being run (generating CHANGELOG, recommendedBump, etc.) is being performed for a pre-major release (<1.0.0).\n This config setting will generally be set by tooling and not a user.
    #[serde(default)]
    pre_major: bool,
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
    /// An optional template directory. The template should be called `template.hbs`. Partials can be used.
    pub(crate) template: Option<PathBuf>,
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
            host: None,
            owner: None,
            repository: None,
            template: None,
        }
    }
}

fn default_header() -> String {
    "# Changelog\n\n".into()
}

fn default_types() -> Vec<Type> {
    vec![
        Type {
            r#type: "feat".into(),
            section: "Features".into(),
            hidden: false,
        },
        Type {
            r#type: "fix".into(),
            section: "Fixes".into(),
            hidden: false,
        },
    ]
}

fn default_commit_url_format() -> String {
    "{{host}}/{{owner}}/{{repository}}/commit/{{hash}}".into()
}

fn default_compare_url_format() -> String {
    "{{host}}/{{owner}}/{{repository}}/compare/{{previousTag}}...{{currentTag}}".into()
}

fn default_issue_url_format() -> String {
    "{{host}}/{{owner}}/{{repository}}/issues/{{id}}".into()
}

fn default_user_url_format() -> String {
    "{{host}}/{{user}}".into()
}

fn default_release_commit_message_format() -> String {
    "chore(release): {{currentTag}}".into()
}

fn default_issue_prefixes() -> Vec<String> {
    vec!["#".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_yaml;

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
              ]
            }"#;
        let value: Config = serde_yaml::from_str(json).unwrap();
        assert_eq!(
            value,
            Config {
                header: "# Changelog\n\n".to_string(),
                types: vec![
                    Type {
                        r#type: "chore".into(),
                        section: "Others".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "revert".into(),
                        section: "Reverts".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "feat".into(),
                        section: "Features".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "fix".into(),
                        section: "Bug Fixes".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "improvement".into(),
                        section: "Feature Improvements".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "docs".into(),
                        section: "Docs".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "style".into(),
                        section: "Styling".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "refactor".into(),
                        section: "Code Refactoring".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "perf".into(),
                        section: "Performance Improvements".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "test".into(),
                        section: "Tests".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "build".into(),
                        section: "Build System".into(),
                        hidden: false
                    },
                    Type {
                        r#type: "ci".into(),
                        section: "CI".into(),
                        hidden: false
                    }
                ],
                pre_major: false,
                commit_url_format: "{{host}}/{{owner}}/{{repository}}/commit/{{hash}}".to_string(),
                compare_url_format:
                    "{{host}}/{{owner}}/{{repository}}/compare/{{previousTag}}...{{currentTag}}"
                        .to_string(),
                issue_url_format: "{{host}}/{{owner}}/{{repository}}/issues/{{id}}".to_string(),
                user_url_format: "{{host}}/{{user}}".to_string(),
                release_commit_message_format: "chore(release): {{currentTag}}".to_string(),
                issue_prefixes: vec!["#".into()],
                host: None,
                owner: None,
                repository: None,
                template: None,
            }
        )
    }
}
