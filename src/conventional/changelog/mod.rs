use crate::Error;
use chrono::NaiveDate;
use handlebars::{no_escape, Handlebars};
use serde::{Deserialize, Serialize};
use std::io;

/// [Conventional Changelog Configuration](https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md)
/// Describes the configuration options supported by conventional-config for upstream tooling.
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
    commit_url_format: String,
    /// A URL representing the comparison between two git SHAs.
    #[serde(default = "default_compare_url_format")]
    compare_url_format: String,
    /// A URL representing the issue format (allowing a different URL format to be swapped in for Gitlab, Bitbucket, etc).
    #[serde(default = "default_issue_url_format")]
    issue_url_format: String,
    /// A URL representing the a user's profile URL on GitHub, Gitlab, etc. This URL is used for substituting @bcoe with https://github.com/bcoe in commit messages.
    #[serde(default = "default_user_url_format")]
    user_url_format: String,
    /// A string to be used to format the auto-generated release commit message.
    #[serde(default = "default_release_commit_message_format")]
    release_commit_message_format: String,
    /// An array of prefixes used to detect references to issues
    #[serde(default = "default_issue_prefixes")]
    pub(crate) issue_prefixes: Vec<String>,
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Type {
    pub(crate) r#type: String,
    #[serde(default)]
    pub(crate) section: String,
    #[serde(default)]
    pub(crate) hidden: bool,
}

const TEMPLATE: &str = include_str!("template.hbs");
const HEADER: &str = include_str!("header.hbs");
const FOOTER: &str = include_str!("footer.hbs");
const COMMIT: &str = include_str!("commit.hbs");

#[derive(Debug, Serialize)]
pub(crate) struct Reference<'a> {
    pub(crate) action: Option<String>,
    pub(crate) owner: &'a str,
    pub(crate) repository: &'a str,
    pub(crate) prefix: String,
    pub(crate) issue: String,
    pub(crate) raw: String,
}

#[derive(Serialize)]
pub(crate) struct Note {
    pub(crate) scope: Option<String>,
    pub(crate) text: String,
}

#[derive(Serialize)]
pub(crate) struct NoteGroup {
    pub(crate) title: String,
    pub(crate) notes: Vec<Note>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitContext<'a> {
    pub(crate) hash: String,
    pub(crate) date: NaiveDate,
    pub(crate) subject: String,
    pub(crate) scope: Option<String>,
    pub(crate) short_hash: String,
    pub(crate) references: Vec<Reference<'a>>,
}

#[derive(Serialize)]
pub(crate) struct CommitGroup<'a> {
    pub(crate) title: &'a str,
    pub(crate) commits: Vec<CommitContext<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Context<'a> {
    #[serde(flatten)]
    pub(crate) context: ContextBase<'a>,
    pub(crate) compare_url_format: String,
    pub(crate) commit_url_format: String,
    pub(crate) issue_url_format: String,
    pub(crate) release_commit_message_format: String,
    pub(crate) user_url_format: String,
    /// `true` if `previousTag` and `currentTag` are truthy.
    pub(crate) link_compare: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextBase<'a> {
    pub(crate) version: &'a str,
    pub(crate) date: Option<NaiveDate>,
    pub(crate) is_patch: bool,
    pub(crate) commit_groups: Vec<CommitGroup<'a>>,
    pub(crate) note_groups: Vec<NoteGroup>,
    pub(crate) previous_tag: &'a str,
    pub(crate) current_tag: &'a str,
}

pub(crate) struct ContextBuilder<'a> {
    handlebars: Handlebars,
    pub(crate) context: ContextBase<'a>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(config: &'a Config) -> Result<ContextBuilder<'a>, Error> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("compare_url_format", config.compare_url_format.as_str())?;
        handlebars
            .register_template_string("commit_url_format", config.commit_url_format.as_str())?;
        handlebars
            .register_template_string("issue_url_format", config.issue_url_format.as_str())?;
        handlebars.register_template_string(
            "release_commit_message_format",
            config.release_commit_message_format.as_str(),
        )?;
        handlebars.register_template_string("user_url_format", config.user_url_format.as_str())?;
        Ok(Self {
            handlebars,
            context: ContextBase {
                version: Default::default(),
                date: Default::default(),
                is_patch: Default::default(),
                commit_groups: Default::default(),
                note_groups: Default::default(),
                previous_tag: "",
                current_tag: "",
            },
        })
    }

    pub fn version(mut self, version: &'a str) -> Self {
        self.context.version = version;
        self
    }

    pub fn date(mut self, date: NaiveDate) -> Self {
        self.context.date = Some(date);
        self
    }

    pub fn is_patch(mut self, is_patch: bool) -> Self {
        self.context.is_patch = is_patch;
        self
    }

    pub fn commit_groups(mut self, commit_groups: Vec<CommitGroup<'a>>) -> Self {
        self.context.commit_groups = commit_groups;
        self
    }

    pub fn note_groups(mut self, note_groups: Vec<NoteGroup>) -> Self {
        self.context.note_groups = note_groups;
        self
    }

    pub fn previous_tag(mut self, previous_tag: &'a str) -> Self {
        self.context.previous_tag = previous_tag;
        self
    }

    pub fn current_tag(mut self, current_tag: &'a str) -> Self {
        self.context.current_tag = current_tag;
        self
    }

    pub fn build(self) -> Result<Context<'a>, Error> {
        let compare_url_format = self
            .handlebars
            .render("compare_url_format", &self.context)?;
        let commit_url_format = self.handlebars.render("commit_url_format", &self.context)?;
        let issue_url_format = self.handlebars.render("issue_url_format", &self.context)?;
        let release_commit_message_format = self
            .handlebars
            .render("release_commit_message_format", &self.context)?;
        let user_url_format = self.handlebars.render("user_url_format", &self.context)?;
        let link_compare = self.context.current_tag != "" && self.context.previous_tag != "";
        Ok(Context {
            context: self.context,
            compare_url_format,
            commit_url_format,
            issue_url_format,
            release_commit_message_format,
            user_url_format,
            link_compare,
        })
    }
}

pub(crate) struct ChangelogWriter<W: io::Write> {
    pub(crate) writer: W,
}

impl<W: io::Write> ChangelogWriter<W> {
    pub(crate) fn write_header(&mut self, header: &str) -> Result<(), Error> {
        write!(self.writer, "{}", header)?;
        Ok(())
    }

    pub fn write_template(&mut self, context: &Context<'_>) -> Result<(), Error> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(no_escape);

        handlebars.register_template_string("template", TEMPLATE)?;
        handlebars.register_partial("header", HEADER)?;
        handlebars.register_partial("commit", COMMIT)?;
        handlebars.register_partial("footer", FOOTER)?;

        let writer = &mut self.writer;
        handlebars.render_to_write("template", context, writer)?;
        Ok(())
    }
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
                issue_prefixes: vec!["#".into()]
            }
        )
    }
}
