mod handlebars;

use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use ::handlebars::Handlebars;
use jiff::civil::Date;
use serde::Serialize;
use walkdir::WalkDir;

use super::config::Config;
use crate::ConvcoError;

const TEMPLATE: &str = include_str!("changelog/template.hbs");
const HEADER: &str = include_str!("changelog/header.hbs");
const FOOTER: &str = include_str!("changelog/footer.hbs");
const COMMIT: &str = include_str!("changelog/commit.hbs");

#[derive(Debug, Serialize)]
pub struct Reference<'a> {
    pub action: Option<String>,
    pub owner: &'a str,
    pub repository: &'a str,
    pub prefix: String,
    pub issue: String,
}

#[derive(Debug, Serialize)]
pub struct Note {
    pub scope: Option<String>,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct NoteGroup {
    pub title: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitContext<'a> {
    pub hash: String,
    pub date: Date,
    pub subject: String,
    pub body: Option<String>,
    pub scope: Option<String>,
    pub short_hash: String,
    pub references: Vec<Reference<'a>>,
}

#[derive(Debug, Serialize)]
pub struct CommitGroup<'a> {
    pub title: &'a str,
    pub commits: Vec<CommitContext<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Context<'a> {
    #[serde(flatten)]
    pub context: ContextBase<'a>,
    pub compare_url_format: String,
    pub release_commit_message_format: String,
    pub user_url_format: String,
    /// `true` if `previousTag` and `currentTag` are truthy.
    pub link_compare: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBase<'a> {
    pub version: Cow<'a, str>,
    pub date: Option<Date>,
    pub is_patch: bool,
    pub commit_groups: Vec<CommitGroup<'a>>,
    pub note_groups: Vec<NoteGroup>,
    pub previous_tag: String,
    pub current_tag: Cow<'a, str>,
    pub host: Option<String>,
    pub owner: Option<String>,
    pub repository: Option<String>,
    pub link_compare: bool,
    pub link_references: bool,
}

pub struct ContextBuilder<'a> {
    handlebars: Handlebars<'a>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(config: &'a Config) -> Result<ContextBuilder<'a>, ConvcoError> {
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

    pub fn build(&self, context_base: ContextBase<'a>) -> Result<Context<'a>, ConvcoError> {
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

pub struct ChangelogWriter<W: io::Write> {
    writer: W,
    handlebars: Handlebars<'static>,
}

impl<W: io::Write> ChangelogWriter<W> {
    pub fn new(template: Option<&Path>, config: &Config, writer: W) -> Result<Self, ConvcoError> {
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

    pub fn write_header(&mut self, header: &str) -> Result<(), ConvcoError> {
        write!(self.writer, "{}", header)?;
        Ok(())
    }

    pub fn write_template(&mut self, context: &Context<'_>) -> Result<(), ConvcoError> {
        let writer = &mut self.writer;
        self.handlebars
            .render_to_write("template", context, writer)
            .map_err(Box::new)?;
        Ok(())
    }
}
