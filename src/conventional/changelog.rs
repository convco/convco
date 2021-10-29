use std::{io, path::Path};

use time::Date;
use handlebars::{no_escape, Handlebars};
use serde::Serialize;

use super::config::Config;
use crate::Error;

const TEMPLATE: &str = include_str!("changelog/template.hbs");
const HEADER: &str = include_str!("changelog/header.hbs");
const FOOTER: &str = include_str!("changelog/footer.hbs");
const COMMIT: &str = include_str!("changelog/commit.hbs");

#[derive(Debug, Serialize)]
pub(crate) struct Reference<'a> {
    pub(crate) action: Option<String>,
    pub(crate) owner: &'a str,
    pub(crate) repository: &'a str,
    pub(crate) prefix: String,
    pub(crate) issue: String,
    pub(crate) raw: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Note {
    pub(crate) scope: Option<String>,
    pub(crate) text: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NoteGroup {
    pub(crate) title: String,
    pub(crate) notes: Vec<Note>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitContext<'a> {
    pub(crate) hash: String,
    pub(crate) date: Date,
    pub(crate) subject: Vec<String>,
    pub(crate) body: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) short_hash: String,
    pub(crate) references: Vec<Reference<'a>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommitGroup<'a> {
    pub(crate) title: &'a str,
    pub(crate) commits: Vec<CommitContext<'a>>,
}

#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextBase<'a> {
    pub(crate) version: &'a str,
    pub(crate) date: Option<Date>,
    pub(crate) is_patch: bool,
    pub(crate) commit_groups: Vec<CommitGroup<'a>>,
    pub(crate) note_groups: Vec<NoteGroup>,
    pub(crate) previous_tag: &'a str,
    pub(crate) current_tag: &'a str,
    pub(crate) host: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) repository: Option<String>,
}

pub(crate) struct ContextBuilder<'a> {
    handlebars: Handlebars<'a>,
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
        Ok(Self { handlebars })
    }

    pub fn build(&self, context_base: ContextBase<'a>) -> Result<Context<'a>, Error> {
        let compare_url_format = self
            .handlebars
            .render("compare_url_format", &context_base)?;
        let commit_url_format = self.handlebars.render("commit_url_format", &context_base)?;
        let issue_url_format = self.handlebars.render("issue_url_format", &context_base)?;
        let release_commit_message_format = self
            .handlebars
            .render("release_commit_message_format", &context_base)?;
        let user_url_format = self.handlebars.render("user_url_format", &context_base)?;
        let link_compare =
            !context_base.current_tag.is_empty() && !context_base.previous_tag.is_empty();
        Ok(Context {
            context: context_base,
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
    writer: W,
    handlebars: Handlebars<'static>,
}

impl<W: io::Write> ChangelogWriter<W> {
    pub(crate) fn new(template: Option<&Path>, writer: W) -> Result<Self, Error> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(no_escape);

        if let Some(path) = template {
            handlebars.register_templates_directory(".hbs", path)?;
        } else {
            handlebars.register_template_string("template", TEMPLATE)?;
            handlebars.register_partial("header", HEADER)?;
            handlebars.register_partial("commit", COMMIT)?;
            handlebars.register_partial("footer", FOOTER)?;
        }

        Ok(Self { writer, handlebars })
    }

    pub(crate) fn write_header(&mut self, header: &str) -> Result<(), Error> {
        write!(self.writer, "{}", header)?;
        Ok(())
    }

    pub(crate) fn write_template(&mut self, context: &Context<'_>) -> Result<(), Error> {
        let writer = &mut self.writer;
        self.handlebars
            .render_to_write("template", context, writer)?;
        Ok(())
    }
}
