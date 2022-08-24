use std::process::{self, ExitStatus};

use handlebars::{no_escape, Handlebars};
use regex::Regex;
use serde::Serialize;

use crate::{
    cli::CommitCommand,
    conventional::{config::Type, CommitParser, Config, ParseError},
    Command, Error,
};

fn read_single_line(
    theme: &impl dialoguer::theme::Theme,
    prompt: &str,
    default: &str,
) -> Result<String, Error> {
    Ok(dialoguer::Input::with_theme(theme)
        .with_prompt(prompt)
        .default(default.to_string())
        .allow_empty(true)
        .interact()?)
}

impl CommitCommand {
    fn commit(&self, msg: String) -> Result<ExitStatus, Error> {
        // build the command
        let mut cmd = process::Command::new("git");
        cmd.args(&["commit", "-m", msg.as_str()]);

        if !self.extra_args.is_empty() {
            cmd.args(&self.extra_args);
        }
        Ok(cmd.status()?)
    }
}

fn read_scope(
    theme: &impl dialoguer::theme::Theme,
    default: &str,
    scope_regex: Regex,
) -> Result<String, Error> {
    let result: String = dialoguer::Input::with_theme(theme)
        .with_prompt("scope")
        .validate_with(move |input: &String| match scope_regex.is_match(input) {
            true => Ok(()),
            false => {
                if input.is_empty() {
                    Ok(())
                } else {
                    Err(format!("scope does not match regex {:?}", scope_regex))
                }
            }
        })
        .default(default.to_string())
        .allow_empty(true)
        .interact()?;
    Ok(result)
}

fn read_description(
    theme: &impl dialoguer::theme::Theme,
    default: String,
) -> Result<String, Error> {
    let result: String = dialoguer::Input::with_theme(theme)
        .with_prompt("description")
        .validate_with(|input: &String| {
            if input.len() < 10 {
                Err("Description needs a length of at least 10 characters")
            } else {
                Ok(())
            }
        })
        .default(default)
        .allow_empty(false)
        .interact()?;
    Ok(result)
}

fn edit_message(msg: &str) -> Result<String, Error> {
    Ok(dialoguer::Editor::new()
        .require_save(false)
        .edit(msg)?
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_owned())
}

#[derive(Serialize)]
struct Dialog {
    r#type: String,
    scope: String,
    description: String,
    body: String,
    breaking: bool,
    breaking_change: String,
    issues: Vec<String>,
    footers: Vec<String>,
}

const BODY_MSG: &str = "# A longer commit body MAY be provided after the short description,\n\
# providing additional contextual information about the code changes.\n\
# The body MUST begin one blank line after the description.\n\
# A commit body is free-form and MAY consist of any number of newline separated paragraphs.\n\
# Lines starting with `#` will be ignored.\n\
# An empty message aborts the commit\n";

impl Dialog {
    fn select_type(
        theme: &impl dialoguer::theme::Theme,
        selected: &str,
        types: &[Type],
    ) -> Result<String, Error> {
        let index = dialoguer::Select::with_theme(theme)
            .with_prompt("type")
            .items(types)
            .default(types.iter().position(|t| t.r#type == selected).unwrap_or(0))
            .interact()?;
        Ok(r#types[index].r#type.clone())
    }

    fn wizard(
        &mut self,
        config: &Config,
        parser: CommitParser,
        interactive: bool,
    ) -> Result<String, Error> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);
        handlebars.register_escape_fn(no_escape);
        let commit_template = match &config.commit_template {
            Some(path) => std::fs::read_to_string(path)?,
            None => include_str!("../conventional/commit/message.hbs").to_owned(),
        };
        handlebars.register_template_string("commit-message", commit_template.as_str())?;
        if !(interactive || self.r#type.is_empty() || self.description.is_empty()) {
            let msg = handlebars.render("commit-message", self)?;
            parser
                .parse(msg.as_str())
                .map(|_| msg)
                .map_err(Error::Parser)
        } else {
            let theme = &dialoguer::theme::ColorfulTheme::default();
            let types = config.types.as_slice();
            let scope_regex = Regex::new(config.scope_regex.as_str()).expect("valid scope regex");
            // make sure that the cursor re-appears when interrupting
            ctrlc::set_handler(move || {
                let term = dialoguer::console::Term::stdout();
                let _ = term.show_cursor();
            })
            .unwrap();
            self.r#type = Self::select_type(theme, self.r#type.as_str(), types)?;
            self.scope = read_scope(theme, self.scope.as_str(), scope_regex)?;
            self.description = read_description(theme, self.description.clone())?;
            self.body = format!("{}\n{}", self.body, BODY_MSG);
            self.breaking_change = read_single_line(
                theme,
                "optional BREAKING change",
                self.breaking_change.as_str(),
            )?;
            self.breaking = self.breaking || !self.breaking_change.is_empty();
            self.issues = read_single_line(
                theme,
                "issues (e.g. #2, #8)",
                self.issues.join(", ").as_str(),
            )?
            .split(|c| c == ' ' || c == ',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_owned())
            .collect();

            loop {
                // finally make message
                let msg = handlebars.render("commit-message", self)?;
                let msg = edit_message(msg.as_str())?;
                match parser.parse(msg.as_str()).map(|_| msg) {
                    Ok(msg) => break Ok(msg),
                    Err(ParseError::EmptyCommitMessage) => break Err(Error::CancelledByUser),
                    Err(e) => {
                        eprintln!("ParseError: {}", e);
                        if !dialoguer::Confirm::new()
                            .with_prompt("Continue?")
                            .interact()?
                        {
                            break Err(Error::CancelledByUser);
                        }
                    }
                }
            }
        }
    }
}

impl Command for CommitCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let r#type = match (
            self.feat,
            self.fix,
            self.build,
            self.chore,
            self.ci,
            self.docs,
            self.style,
            self.refactor,
            self.perf,
            self.test,
            self.r#type.as_ref(),
        ) {
            (true, false, false, false, false, false, false, false, false, false, None) => {
                "feat".to_string()
            }
            (false, true, false, false, false, false, false, false, false, false, None) => {
                "fix".to_string()
            }
            (false, false, true, false, false, false, false, false, false, false, None) => {
                "build".to_string()
            }
            (false, false, false, true, false, false, false, false, false, false, None) => {
                "chore".to_string()
            }
            (false, false, false, false, true, false, false, false, false, false, None) => {
                "ci".to_string()
            }
            (false, false, false, false, false, true, false, false, false, false, None) => {
                "docs".to_string()
            }
            (false, false, false, false, false, false, true, false, false, false, None) => {
                "style".to_string()
            }
            (false, false, false, false, false, false, false, true, false, false, None) => {
                "refactor".to_string()
            }
            (false, false, false, false, false, false, false, false, true, false, None) => {
                "perf".to_string()
            }
            (false, false, false, false, false, false, false, false, false, true, None) => {
                "test".to_string()
            }
            (false, false, false, false, false, false, false, false, false, false, feat) => {
                feat.cloned().unwrap_or_default()
            }
            _ => Default::default(),
        };
        let parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .build();
        let description = self.message.first().cloned().unwrap_or_default();
        let body = self
            .message
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<String>>()
            .join("\n\n");
        let msg = Dialog {
            r#type,
            scope: self.scope.as_ref().cloned().unwrap_or_default(),
            description,
            body,
            breaking: self.breaking,
            breaking_change: String::new(),
            issues: Vec::new(),
            footers: self
                .footers
                .iter()
                .map(|f| format!("{}: {}", f.0, f.1))
                .collect(),
        }
        .wizard(&config, parser, self.interactive)?;

        self.commit(msg)?;
        Ok(())
    }
}
