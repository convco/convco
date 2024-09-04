mod handlebars;

use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use ::handlebars::Handlebars;
use serde::Serialize;
use time::Date;
use walkdir::WalkDir;

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
}

#[derive(Debug, Serialize)]
pub(crate) struct Note {
    pub(crate) scope: Option<String>,
    pub(crate) text: String,
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
    pub(crate) subject: String,
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
    pub(crate) release_commit_message_format: String,
    pub(crate) user_url_format: String,
    /// `true` if `previousTag` and `currentTag` are truthy.
    pub(crate) link_compare: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextBase<'a> {
    pub(crate) version: Cow<'a, str>,
    pub(crate) date: Option<Date>,
    pub(crate) is_patch: bool,
    pub(crate) commit_groups: Vec<CommitGroup<'a>>,
    pub(crate) note_groups: Vec<NoteGroup>,
    pub(crate) previous_tag: &'a str,
    pub(crate) current_tag: Cow<'a, str>,
    pub(crate) host: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) link_compare: bool,
    pub(crate) link_references: bool,
}

pub(crate) struct ContextBuilder<'a> {
    handlebars: Handlebars<'a>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(config: &'a Config) -> Result<ContextBuilder<'a>, Error> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("compare_url_format", config.compare_url_format.as_str())
            .map_err(Box::new)?;
        handlebars
            .register_template_string(
                "release_commit_message_format",
                config.release_commit_message_format.as_str(),
            )
            .map_err(Box::new)?;
        handlebars
            .register_template_string("user_url_format", config.user_url_format.as_str())
            .map_err(Box::new)?;
        Ok(Self { handlebars })
    }

    pub fn build(&self, context_base: ContextBase<'a>) -> Result<Context<'a>, Error> {
        let compare_url_format = self
            .handlebars
            .render("compare_url_format", &context_base)
            .map_err(Box::new)?;
        let release_commit_message_format = self
            .handlebars
            .render("release_commit_message_format", &context_base)
            .map_err(Box::new)?;
        let user_url_format = self
            .handlebars
            .render("user_url_format", &context_base)
            .map_err(Box::new)?;
        let link_compare = context_base.link_compare
            && !context_base.current_tag.is_empty()
            && !context_base.previous_tag.is_empty();
        Ok(Context {
            context: context_base,
            compare_url_format,
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
    pub(crate) fn new(template: Option<&Path>, config: &Config, writer: W) -> Result<Self, Error> {
        let mut handlebars = self::handlebars::new(config.line_length, config.wrap_disabled);

        fn replace_url_formats(tpl_str: &str, config: &Config) -> String {
            tpl_str
                .replace("{{commitUrlFormat}}", config.commit_url_format.as_str())
                .replace("{{issueUrlFormat}}", config.issue_url_format.as_str())
        }

        if let Some(path) = template {
            for entry in WalkDir::new(path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_entry(|e| e.file_name().to_string_lossy().ends_with(".hbs"))
                .filter_map(|e| e.ok())
            {
                if entry.metadata().unwrap().is_file() {
                    let mut reader = BufReader::new(File::open(entry.path())?);
                    let mut tpl_str = String::new();
                    reader.read_to_string(&mut tpl_str)?;
                    let tpl_str = replace_url_formats(tpl_str.as_str(), config);

                    let name = entry.file_name().to_string_lossy();
                    let name = name.trim_end_matches(".hbs");

                    handlebars
                        .register_template_string(name, tpl_str)
                        .map_err(Box::new)?;
                }
            }
        } else {
            handlebars
                .register_template_string("template", replace_url_formats(TEMPLATE, config))
                .map_err(Box::new)?;
            handlebars
                .register_partial("header", replace_url_formats(HEADER, config))
                .map_err(Box::new)?;
            handlebars
                .register_partial("commit", replace_url_formats(COMMIT, config))
                .map_err(Box::new)?;
            handlebars
                .register_partial("footer", replace_url_formats(FOOTER, config))
                .map_err(Box::new)?;
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
            .render_to_write("template", context, writer)
            .map_err(Box::new)?;
        Ok(())
    }
}
