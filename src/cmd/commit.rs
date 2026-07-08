use std::{
    collections::HashSet,
    path::PathBuf,
    process::{self, ExitStatus},
    sync::Mutex,
};

use convco::{
    open_repo, strip::Strip, CommitParser, Config, ConvcoError, ParseError, Repo, RevWalkOptions,
    Type,
};
use dialoguer::{BasicHistory, Completion, History};
use handlebars::{no_escape, Handlebars};
use regex::Regex;
use serde::Serialize;

use super::Command;
use crate::cli::CommitCommand;

fn read_single_line(
    theme: &impl dialoguer::theme::Theme,
    prompt: &str,
    default: &str,
) -> Result<String, ConvcoError> {
    Ok(dialoguer::Input::with_theme(theme)
        .with_prompt(prompt)
        .default(default.to_string())
        .allow_empty(true)
        .interact_text()?)
}

impl CommitCommand {
    fn commit(&self, msg: &str) -> Result<ExitStatus, ConvcoError> {
        // build the command
        let mut cmd = process::Command::new("git");
        cmd.args(["commit", "-m", msg]);

        if !self.extra_args.is_empty() {
            cmd.args(&self.extra_args);
        }
        Ok(cmd.status()?)
    }

    fn intend_to_add(&self, paths: &[PathBuf]) -> Result<ExitStatus, ConvcoError> {
        let mut cmd = process::Command::new("git");
        Ok(cmd.args(["add", "-N"]).args(paths).status()?)
    }

    fn patch(&self) -> Result<ExitStatus, ConvcoError> {
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
            Err(ConvcoError::GitCommitFailed(exit_status))?;
        };
        Ok(())
    }
}

fn read_scope(
    theme: &impl dialoguer::theme::Theme,
    default: &str,
    scope_regex: Regex,
    scopes: &[String],
) -> Result<String, ConvcoError> {
    let mut history = scope_history(scopes);
    let completion = ScopeCompletion::new(scopes);
    let result: String = dialoguer::Input::with_theme(theme)
        .with_prompt("scope")
        .history_with(&mut history)
        .completion_with(&completion)
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
        .interact_text()?;
    Ok(result)
}

struct ScopeCompletion {
    scopes: Vec<String>,
    state: Mutex<Option<ScopeCompletionState>>,
}

struct ScopeCompletionState {
    prefix: String,
    matches: Vec<String>,
    index: usize,
}

impl ScopeCompletion {
    fn new(scopes: &[String]) -> Self {
        Self {
            scopes: scopes.to_owned(),
            state: Mutex::new(None),
        }
    }

    fn matches(&self, input: &str) -> Vec<String> {
        self.scopes
            .iter()
            .filter(|scope| scope.starts_with(input))
            .cloned()
            .collect()
    }
}

impl Completion for ScopeCompletion {
    fn get(&self, input: &str) -> Option<String> {
        let mut state = self.state.lock().expect("scope completion state lock");

        if let Some(state) = state.as_mut() {
            let current = state.matches.get(state.index)?;
            if input == current || input == state.prefix {
                state.index = (state.index + 1) % state.matches.len();
                return state.matches.get(state.index).cloned();
            }
        }

        let matches = self.matches(input);
        match matches.as_slice() {
            [] => {
                *state = None;
                None
            }
            [scope] => {
                *state = None;
                Some(scope.to_string())
            }
            _ => {
                if input.is_empty() {
                    let first_match = matches.first().cloned();
                    *state = Some(ScopeCompletionState {
                        prefix: input.to_string(),
                        matches,
                        index: 0,
                    });
                    return first_match;
                }

                let common_prefix = common_prefix(&matches);
                if common_prefix.len() > input.len() {
                    *state = None;
                    Some(common_prefix)
                } else {
                    let first_match = matches.first().cloned();
                    *state = Some(ScopeCompletionState {
                        prefix: input.to_string(),
                        matches,
                        index: 0,
                    });
                    first_match
                }
            }
        }
    }
}

fn common_prefix(scopes: &[String]) -> String {
    let Some(first) = scopes.first() else {
        return String::new();
    };

    let mut end = first.len();
    for scope in scopes.iter().skip(1) {
        end = first
            .char_indices()
            .take_while(|(index, ch)| {
                scope
                    .get(*index..)
                    .and_then(|suffix| suffix.chars().next())
                    .is_some_and(|other| other == *ch)
            })
            .map(|(index, ch)| index + ch.len_utf8())
            .last()
            .unwrap_or(0)
            .min(end);
    }

    first[..end].to_string()
}

fn scope_history(scopes: &[String]) -> BasicHistory {
    let mut history = BasicHistory::new()
        .max_entries(scopes.len())
        .no_duplicates(true);
    for scope in scopes.iter().rev() {
        history.write(scope);
    }
    history
}

fn read_description(
    theme: &impl dialoguer::theme::Theme,
    default: String,
    min_length: usize,
    max_length: usize,
) -> Result<String, ConvcoError> {
    let result: String = dialoguer::Input::with_theme(theme)
        .with_prompt("description")
        .validate_with(|input: &String| {
            if input.len() < min_length {
                Err(format!(
                    "Description needs a length of at least {min_length} characters"
                ))
            } else if input.len() > max_length {
                Err(format!(
                    "Description needs a length of at most {max_length} characters"
                ))
            } else {
                Ok(())
            }
        })
        .default(default)
        .allow_empty(false)
        .interact_text()?;
    Ok(result)
}

fn edit_message(msg: &str) -> Result<String, ConvcoError> {
    Ok(dialoguer::Editor::new()
        .require_save(false)
        .edit(msg)?
        .unwrap_or_default()
        .strip())
}

fn edit_loop(msg: &str, parser: &CommitParser, types: &[String]) -> Result<String, ConvcoError> {
    let mut edit_msg = msg.to_owned();
    loop {
        edit_msg = edit_message(&edit_msg)?;
        match parser.parse(&edit_msg) {
            Ok(commit) => {
                if !types.contains(&commit.r#type) {
                    eprintln!(
                        "ParseError: {}",
                        ConvcoError::Type {
                            wrong_type: commit.r#type.to_string(),
                        }
                    );
                    if !dialoguer::Confirm::new()
                        .with_prompt("Continue?")
                        .interact()?
                    {
                        break Err(ConvcoError::CancelledByUser);
                    }
                } else {
                    break Ok(edit_msg);
                }
            }
            Err(ParseError::EmptyConventionalCommitMessage) => {
                break Err(ConvcoError::CancelledByUser);
            }
            Err(e) => {
                eprintln!("ParseError: {}", e);
                if !dialoguer::Confirm::new()
                    .with_prompt("Continue?")
                    .interact()?
                {
                    break Err(ConvcoError::CancelledByUser);
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
    ) -> Result<String, ConvcoError> {
        let index = dialoguer::FuzzySelect::with_theme(theme)
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
        scopes: &[String],
    ) -> Result<String, ConvcoError> {
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
                .map_err(ConvcoError::Parser)
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
            self.scope = read_scope(theme, self.scope.as_str(), scope_regex, scopes)?;
            self.description = read_description(
                theme,
                self.description.clone(),
                config.description.length.min.unwrap_or(0),
                config.description.length.max.unwrap_or(usize::MAX),
            )?;
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
            .split([' ', ','])
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
        let scopes = collect_commit_scopes(&parser, self.scope_history_limit).unwrap_or_default();
        let types = config_types_to_conventional(&config.types);
        if !is_git_editor {
            if !self.intent_to_add.is_empty() {
                self.intend_to_add(self.intent_to_add.as_slice())?;
            }
            if self.patch {
                self.patch()?;
            }
        }
        if let Ok(ref msg) = std::fs::read_to_string(commit_editmsg_path) {
            if parser.parse(msg).is_ok() {
                if is_git_editor {
                    let msg = edit_loop(msg, &parser, &types)?;
                    std::fs::write(commit_editmsg_path, msg)?;
                    return Ok(());
                }
                loop {
                    println!("Recovery commit message found:\n\n{msg}\n",);

                    let input: String = dialoguer::Input::new()
                        .with_prompt("Do you want to (a)ccept/(e)dit/(r)eject?")
                        .interact_text()
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
        .wizard(&config, parser, self.interactive, &scopes)?;

        std::fs::write(commit_editmsg_path, &msg)?;
        if !is_git_editor {
            self.commit_msg_and_remove_file(&msg, commit_editmsg_path)?;
        }

        Ok(())
    }
}

fn config_types_to_conventional(types: &[Type]) -> Vec<String> {
    types
        .iter()
        .map(|ty| ty.r#type.as_str())
        .map(String::from)
        .collect()
}

fn collect_commit_scopes(
    parser: &CommitParser,
    scope_history_limit: usize,
) -> Result<Vec<String>, ConvcoError> {
    let repo = open_repo()?;
    collect_commit_scopes_from_repo(&repo, parser, scope_history_limit)
}

fn collect_commit_scopes_from_repo<'repo, R>(
    repo: &'repo R,
    parser: &'repo CommitParser,
    scope_history_limit: usize,
) -> Result<Vec<String>, ConvcoError>
where
    R: Repo<'repo>,
{
    let Ok(head) = Repo::revparse_single(repo, "HEAD") else {
        return Ok(Vec::new());
    };
    let options = RevWalkOptions {
        from_rev: Vec::new(),
        to_rev: head,
        first_parent: false,
        no_merge_commits: false,
        no_revert_commits: true,
        paths: Vec::new(),
        parser,
    };

    let mut seen = HashSet::new();
    let mut scopes = Vec::new();

    for commit in Repo::revwalk(repo, options)?.take(scope_history_limit) {
        if let Ok(commit) = commit {
            push_scope(&mut scopes, &mut seen, commit.conventional_commit.scope);
        }
    }

    Ok(scopes)
}

fn push_scope(scopes: &mut Vec<String>, seen: &mut HashSet<String>, scope: Option<String>) {
    if let Some(scope) = scope {
        if seen.insert(scope.clone()) {
            scopes.push(scope);
        }
    }
}

fn get_default_commit_msg_path() -> Result<PathBuf, ConvcoError> {
    let repo = open_repo()?;
    Ok(repo.path().join("CONVCO_MSG"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_completion_returns_unique_prefix_match() {
        let completion = ScopeCompletion::new(&["commit".into(), "changelog".into()]);

        assert_eq!(completion.get("com"), Some("commit".into()));
    }

    #[test]
    fn scope_completion_extends_ambiguous_prefix_to_common_prefix() {
        let completion = ScopeCompletion::new(&["commit-parser".into(), "commit-ui".into()]);

        assert_eq!(completion.get("c"), Some("commit-".into()));
    }

    #[test]
    fn scope_completion_cycles_ambiguous_prefix_without_progress() {
        let completion = ScopeCompletion::new(&["commit".into(), "config".into()]);

        assert_eq!(completion.get("co"), Some("commit".into()));
        assert_eq!(completion.get("commit"), Some("config".into()));
        assert_eq!(completion.get("config"), Some("commit".into()));
    }

    #[test]
    fn scope_completion_cycles_scopes_with_same_prefix() {
        let completion = ScopeCompletion::new(&["changelog".into(), "check".into()]);

        assert_eq!(completion.get("ch"), Some("changelog".into()));
        assert_eq!(completion.get("changelog"), Some("check".into()));
        assert_eq!(completion.get("check"), Some("changelog".into()));
    }

    #[test]
    fn scope_completion_ignores_unknown_prefix() {
        let completion = ScopeCompletion::new(&["commit".into()]);

        assert_eq!(completion.get("parser"), None);
    }

    #[test]
    fn scope_completion_ignores_empty_input() {
        let completion = ScopeCompletion::new(&["commit".into()]);

        assert_eq!(completion.get(""), Some("commit".into()));
    }

    #[test]
    fn scope_completion_cycles_from_empty_input() {
        let completion = ScopeCompletion::new(&["commit".into(), "check".into()]);

        assert_eq!(completion.get(""), Some("commit".into()));
        assert_eq!(completion.get("commit"), Some("check".into()));
        assert_eq!(completion.get("check"), Some("commit".into()));
    }

    #[test]
    fn scope_completion_ignores_empty_input_without_scopes() {
        let completion = ScopeCompletion::new(&[]);

        assert_eq!(completion.get(""), None);
    }

    #[test]
    fn scope_history_preserves_most_recent_first_order() {
        let history = scope_history(&["commit".into(), "check".into()]);

        assert_eq!(
            <BasicHistory as History<String>>::read(&history, 0),
            Some("commit".into())
        );
        assert_eq!(
            <BasicHistory as History<String>>::read(&history, 1),
            Some("check".into())
        );
        assert_eq!(<BasicHistory as History<String>>::read(&history, 2), None);
    }

    #[test]
    fn push_scope_deduplicates_and_ignores_empty_scopes() {
        let mut scopes = Vec::new();
        let mut seen = HashSet::new();

        push_scope(&mut scopes, &mut seen, Some("commit".into()));
        push_scope(&mut scopes, &mut seen, None);
        push_scope(&mut scopes, &mut seen, Some("check".into()));
        push_scope(&mut scopes, &mut seen, Some("commit".into()));

        assert_eq!(scopes, vec!["commit", "check"]);
    }
}
