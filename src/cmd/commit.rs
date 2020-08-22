use crate::{
    cli::CommitCommand,
    conventional::{CommitParser, Config},
    Command, Error,
};
use std::{
    io,
    io::{Read, Write},
    process::ExitStatus,
};

fn read_single_line(prompt: &str) -> Result<String, Error> {
    let mut out = io::stdout();
    write!(out, "{}", prompt)?;
    out.flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    // remove the newline
    Ok(value.trim().to_owned())
}

fn read_multi_line(prompt: &str) -> Result<String, Error> {
    let mut out = io::stdout();
    writeln!(out, "{}", prompt)?;
    writeln!(out, "Press CTRL+D to stop")?;
    out.flush()?;
    let mut value = String::new();
    io::stdin().read_to_string(&mut value)?;
    // remove the newlines
    Ok(value.trim().to_owned())
}

impl CommitCommand {
    fn type_as_string(&self) -> &str {
        if self.build {
            "build"
        } else if self.chore {
            "chore"
        } else if self.ci {
            "ci"
        } else if self.docs {
            "docs"
        } else if self.feat {
            "feat"
        } else if self.fix {
            "fix"
        } else if self.perf {
            "perf"
        } else if self.refactor {
            "refactor"
        } else if self.style {
            "style"
        } else if self.test {
            "test"
        } else {
            unreachable!()
        }
    }

    fn commit(
        &self,
        scope: String,
        description: String,
        body: String,
        breaking_change: String,
        issues: String,
        parser: CommitParser,
    ) -> Result<ExitStatus, Error> {
        let mut msg = self.type_as_string().to_owned();
        if !scope.is_empty() {
            msg.push('(');
            msg.push_str(scope.as_str());
            msg.push(')');
        }
        if self.breaking || !breaking_change.is_empty() {
            msg.push('!');
        }
        msg.push_str(": ");
        msg.push_str(description.as_str());
        if !body.is_empty() {
            msg.push_str("\n\n");
            msg.push_str(body.as_str())
        }
        if !breaking_change.is_empty() {
            msg.push_str("\n\n");
            msg.push_str(format!("BREAKING CHANGE: {}", breaking_change).as_str());
        }
        if !issues.is_empty() {
            msg.push_str("\n\n");
            msg.push_str(format!("Refs: {}", issues).as_str());
        }
        // validate by parsing
        parser
            .parse(msg.as_str())
            .expect("Matches conventional commit");
        // build the command
        let mut cmd = std::process::Command::new("git");
        cmd.args(&["commit", "-m", msg.as_str()]);

        if !self.extra_args.is_empty() {
            cmd.args(&self.extra_args);
        }
        Ok(cmd.status()?)
    }
}

impl Command for CommitCommand {
    fn exec(&self, config: Config) -> Result<(), Error> {
        let scope = read_single_line("optional scope: ")?;
        let description = read_single_line("description: ")?;
        let body = read_multi_line("optional body:")?;
        let breaking_change = read_single_line("optional BREAKING CHANGE: ")?;
        let issues = read_single_line("optional issues (e.g. #2, #8): ")?;
        let parser = CommitParser::builder()
            .scope_regex(config.scope_regex)
            .build();
        self.commit(scope, description, body, breaking_change, issues, parser)?;
        Ok(())
    }
}
