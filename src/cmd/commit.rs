use std::process::{self, ExitStatus};

use regex::Regex;

use crate::{
    cli::CommitCommand,
    conventional::{config::Type, CommitParser, Config},
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

fn make_commit_message(
    Dialog {
        r#type,
        scope,
        description,
        body,
        breaking_change,
        issues,
    }: &Dialog,
    breaking: bool,
) -> String {
    let mut msg = r#type.to_string();
    if !scope.trim().is_empty() {
        msg.push('(');
        msg.push_str(scope.trim());
        msg.push(')');
    }
    if breaking || !breaking_change.trim().is_empty() {
        msg.push('!');
    }
    msg.push_str(": ");
    msg.push_str(description.trim());
    if !body.is_empty() {
        msg.push_str("\n\n");
        msg.push_str(body.trim())
    }
    if !breaking_change.trim().is_empty() {
        msg.push_str("\n\n");
        msg.push_str(format!("BREAKING CHANGE: {}", breaking_change.trim()).as_str());
    }
    if !issues.trim().is_empty() {
        msg.push_str("\n\n");
        msg.push_str(format!("Refs: {}", issues.trim()).as_str());
    }
    msg
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
        .require_save(true)
        .edit(msg)?
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_owned())
}

struct Dialog {
    r#type: String,
    scope: String,
    description: String,
    body: String,
    breaking_change: String,
    issues: String,
}

impl Default for Dialog {
    fn default() -> Self {
        Self {
            r#type: String::default(),
            scope: String::default(),
            description: String::default(),
            body: "# A longer commit body MAY be provided after the short description, \n\
                   # providing additional contextual information about the code changes. \n\
                   # The body MUST begin one blank line after the description. \n\
                   # A commit body is free-form and MAY consist of any number of newline separated paragraphs.\n".to_string(),
            breaking_change: String::default(),
            issues: String::default(),
        }
    }
}

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

    // Prompt all
    fn wizard(
        config: &Config,
        parser: CommitParser,
        r#type: Option<String>,
        breaking: bool,
    ) -> Result<String, Error> {
        let mut dialog = Self::default();
        let theme = &dialoguer::theme::ColorfulTheme::default();
        let types = config.types.as_slice();
        let scope_regex = Regex::new(config.scope_regex.as_str()).expect("valid scope regex");

        // type
        let current_type = dialog.r#type.as_str();
        match (r#type.as_ref(), current_type) {
            (Some(t), "") if !t.is_empty() => dialog.r#type = t.to_owned(),
            (_, t) => {
                dialog.r#type = Self::select_type(theme, t, types)?;
            }
        }
        // scope
        dialog.scope = read_scope(theme, dialog.scope.as_ref(), scope_regex)?;
        // description
        dialog.description = read_description(theme, dialog.description)?;
        // breaking change
        dialog.breaking_change = read_single_line(
            theme,
            "optional BREAKING change",
            dialog.breaking_change.as_str(),
        )?;
        // issues
        dialog.issues = read_single_line(theme, "issues (e.g. #2, #8)", dialog.issues.as_str())?;

        loop {
            // finally make message
            let msg = make_commit_message(&dialog, breaking);
            let msg = edit_message(msg.as_str())?;
            match parser.parse(msg.as_str()).map(|_| msg) {
                Ok(msg) => break Ok(msg),
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

impl Command for CommitCommand {
    fn exec(&self, config: Config) -> Result<(), Error> {
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
        ) {
            (true, false, false, false, false, false, false, false, false, false) => {
                Some("feat".to_string())
            }
            (false, true, false, false, false, false, false, false, false, false) => {
                Some("fix".to_string())
            }
            (false, false, true, false, false, false, false, false, false, false) => {
                Some("build".to_string())
            }
            (false, false, false, true, false, false, false, false, false, false) => {
                Some("chore".to_string())
            }
            (false, false, false, false, true, false, false, false, false, false) => {
                Some("ci".to_string())
            }
            (false, false, false, false, false, true, false, false, false, false) => {
                Some("docs".to_string())
            }
            (false, false, false, false, false, false, true, false, false, false) => {
                Some("style".to_string())
            }
            (false, false, false, false, false, false, false, true, false, false) => {
                Some("refactor".to_string())
            }
            (false, false, false, false, false, false, false, false, true, false) => {
                Some("perf".to_string())
            }
            (false, false, false, false, false, false, false, false, false, true) => {
                Some("test".to_string())
            }
            _ => None,
        };
        let parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .build();
        let msg = Dialog::wizard(&config, parser, r#type, self.breaking)?;

        self.commit(msg)?;
        Ok(())
    }
}
