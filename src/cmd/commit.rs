use std::{
    path::PathBuf,
    process::{self, ExitStatus},
};

use handlebars::{no_escape, Handlebars};
use regex::Regex;
use serde::Serialize;

use crate::{
    cli::CommitCommand,
    conventional::{config::Type, CommitParser, Config, ParseError},
    strip::Strip,
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
    fn commit(&self, msg: &str) -> Result<ExitStatus, Error> {
        // build the command
        let mut cmd = process::Command::new("git");
        cmd.args(["commit", "-m", msg]);

        if !self.extra_args.is_empty() {
            cmd.args(&self.extra_args);
        }
        Ok(cmd.status()?)
    }

    fn intend_to_add(&self, paths: &[PathBuf]) -> Result<ExitStatus, Error> {
        let mut cmd = process::Command::new("git");
        Ok(cmd.args(["add", "-N"]).args(paths).status()?)
    }

    fn patch(&self) -> Result<ExitStatus, Error> {
        let mut cmd = process::Command::new("git");
        Ok(cmd.args(["add", "-p"]).status()?)
    }

    fn commit_msg_and_remove_file(
        &self,
        msg: &str,
        commit_editmsg: &std::path::Path,
    ) -> Result<(), anyhow::Error> {
        let exit_status = self.commit(msg)?;
        if exit_status.success() {
            std::fs::remove_file(commit_editmsg)?;
        } else {
            Err(Error::GitCommitFailed(exit_status))?;
        };
        Ok(())
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
        .strip())
}

fn edit_loop(
    msg: &str,
    parser: &CommitParser,
    types: &[crate::conventional::Type],
) -> Result<String, Error> {
    let mut edit_msg = msg.to_owned();
    loop {
        edit_msg = edit_message(&edit_msg)?;
        match parser.parse(&edit_msg) {
            Ok(commit) => {
                if !types.contains(&commit.r#type) {
                    eprintln!(
                        "ParseError: {}",
                        Error::Type {
                            wrong_type: commit.r#type.to_string(),
                        }
                    );
                    if !dialoguer::Confirm::new()
                        .with_prompt("Continue?")
                        .interact()?
                    {
                        break Err(Error::CancelledByUser);
                    }
                } else {
                    break Ok(edit_msg);
                }
            }
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
        handlebars
            .register_template_string("commit-message", commit_template.as_str())
            .map_err(Box::new)?;
        if !(interactive || self.r#type.is_empty() || self.description.is_empty()) {
            let msg = handlebars
                .render("commit-message", self)
                .map_err(Box::new)?;
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
                std::process::exit(1);
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
            // finally make message
            let msg = handlebars
                .render("commit-message", self)
                .map_err(Box::new)?;
            edit_loop(&msg, &parser, &config_types_to_conventional(types))
        }
    }
}

impl Command for CommitCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let commit_editmsg = match &self.commit_msg_path {
            Some(path) => path.to_owned(),
            None => get_default_commit_msg_path()?,
        };
        let commit_editmsg_path = commit_editmsg.as_path();
        let is_git_editor = commit_editmsg_path.ends_with("COMMIT_EDITMSG");
        let parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .build();
        let types = config_types_to_conventional(&config.types);
        if !is_git_editor {
            if !self.intent_to_add.is_empty() {
                self.intend_to_add(self.intent_to_add.as_slice())?;
            }
            if self.patch {
                self.patch()?;
            }
            if let Ok(ref msg) = std::fs::read_to_string(commit_editmsg_path) {
                if parser.parse(msg).is_ok() {
                    loop {
                        println!("Recovery commit message found:\n\n{msg}\n",);
                        let input: String = dialoguer::Input::new()
                            .with_prompt("Do you want to (a)ccept/(e)dit/(r)eject?")
                            .interact()
                            .unwrap();
                        match input.as_str() {
                            "a" | "accept" => {
                                self.commit_msg_and_remove_file(msg, commit_editmsg_path)?;
                                return Ok(());
                            }
                            "e" | "edit" => {
                                let msg = edit_loop(msg, &parser, &types)?;
                                self.commit_msg_and_remove_file(&msg, commit_editmsg_path)?;
                            }
                            "r" | "reject" => break,
                            _ => continue,
                        }
                    }
                }
            }
        }
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

        std::fs::write(commit_editmsg_path, &msg)?;
        if !is_git_editor {
            self.commit_msg_and_remove_file(&msg, commit_editmsg_path)?;
        }

        Ok(())
    }
}

fn config_types_to_conventional(types: &[Type]) -> Vec<crate::conventional::Type> {
    types
        .iter()
        .map(|ty| ty.r#type.as_str())
        .map(crate::conventional::Type::from)
        .collect()
}

fn get_default_commit_msg_path() -> Result<PathBuf, Error> {
    let repo = git2::Repository::open_from_env()?;
    Ok(repo.path().join("CONVCO_MSG"))
}
