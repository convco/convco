use std::io::{stdin, Read};

use conventional::Config;
use git2::Repository;

use crate::{
    cli::CheckCommand,
    cmd::Command,
    conventional,
    git::{filter_merge_commits, filter_revert_commits},
    strip::Strip,
    Error,
};

fn print_fail(msg: &str, short_id: &str, e: Error) -> bool {
    let first_line = msg.lines().next().unwrap_or("");
    let short_msg: String = first_line.chars().take(40).collect();
    if first_line.len() > 40 {
        println!("FAIL  {}  {}  {}...", short_id, e, short_msg)
    } else {
        println!("FAIL  {}  {}  {}", short_id, e, short_msg)
    }
    false
}

fn print_wrong_type(msg: &str, short_id: &str, commit_type: String) -> bool {
    print_fail(
        msg,
        short_id,
        Error::Type {
            wrong_type: commit_type.to_string(),
        },
    )
}

fn print_check(
    msg: &str,
    short_id: &str,
    parser: &conventional::CommitParser,
    types: &[String],
) -> bool {
    let msg_parsed = parser.parse(msg);

    match msg_parsed {
        Err(e) => print_fail(msg, short_id, e.into()),
        Ok(commit) if !types.contains(&commit.r#type) => {
            print_wrong_type(msg, short_id, commit.r#type)
        }
        _ => true,
    }
}

impl Command for CheckCommand {
    fn exec(&self, mut config: Config) -> anyhow::Result<()> {
        if self.merges {
            config.merges = true;
        }
        if self.first_parent {
            config.first_parent = true;
        }

        let mut total = 0;
        let mut fail = 0;

        let parser = conventional::CommitParser::builder()
            .scope_regex(config.scope_regex)
            .strip_regex(config.strip_regex)
            .build();
        let types: Vec<String> = config
            .types
            .iter()
            .map(|ty| ty.r#type.as_str())
            .map(String::from)
            .collect();

        let Config { merges, .. } = config;

        if self.from_stdin {
            let mut stdin = stdin().lock();
            let mut commit_msg = String::new();
            stdin.read_to_string(&mut commit_msg)?;
            if self.strip {
                commit_msg = commit_msg.strip();
            }
            let is_conventional = print_check(commit_msg.as_str(), "-", &parser, &types);
            match is_conventional {
                true => return Ok(()),
                false => return Err(Error::Check)?,
            }
        }

        let repo = Repository::open_from_env()?;
        let mut revwalk = repo.revwalk()?;
        if config.first_parent {
            revwalk.simplify_first_parent()?;
        }
        let rev = match self.rev.as_ref() {
            Some(rev) if !rev.is_empty() => rev.as_str(),
            _ => "HEAD",
        };

        if rev.contains("..") {
            revwalk.push_range(rev)?;
        } else {
            let oid = repo.revparse_single(rev)?.id();
            revwalk.push(oid)?;
        }

        for commit in revwalk
            .flatten()
            .flat_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| filter_merge_commits(commit, merges))
            .filter(|commit| filter_revert_commits(commit, self.ignore_reverts))
            .take(self.number.unwrap_or(std::usize::MAX))
        {
            total += 1;
            let msg = std::str::from_utf8(commit.message_bytes()).expect("valid utf-8 message");
            let short_id = commit.as_object().short_id().unwrap();
            let short_id = short_id.as_str().expect("short id");
            fail += u32::from(!print_check(msg, short_id, &parser, &types));
        }
        if fail == 0 {
            match total {
                0 => println!("no commits checked"),
                1 => println!("no errors in {} commit", total),
                _ => println!("no errors in {} commits", total),
            }
            Ok(())
        } else {
            println!("\n{}/{} failed", fail, total);
            Err(Error::Check)?
        }
    }
}
