use conventional::Config;
use git2::{Commit, Repository};

use crate::{
    cli::CheckCommand,
    cmd::Command,
    conventional::{self, Type},
    git::{filter_merge_commits, filter_revert_commits},
    Error,
};

fn print_check(commit: &Commit<'_>, parser: &conventional::CommitParser, types: &[Type]) -> bool {
    let msg = std::str::from_utf8(commit.message_bytes()).expect("valid utf-8 message");
    let short_id = commit.as_object().short_id().unwrap();
    let short_id = short_id.as_str().expect("short id");
    let msg_parsed = parser.parse(msg);
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
    match msg_parsed {
        Err(e) => print_fail(msg, short_id, e.into()),
        Ok(commit) if !types.contains(&commit.r#type) => print_fail(
            msg,
            short_id,
            Error::Type {
                wrong_type: commit.r#type.to_string(),
            },
        ),
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
        let repo = Repository::open_from_env()?;
        let mut revwalk = repo.revwalk()?;
        if config.first_parent {
            revwalk.simplify_first_parent()?;
        }
        if self.rev.contains("..") {
            revwalk.push_range(self.rev.as_str())?;
        } else {
            revwalk.push_ref(self.rev.as_str())?;
        }

        let mut total = 0;
        let mut fail = 0;

        let parser = conventional::CommitParser::builder()
            .scope_regex(config.scope_regex)
            .build();
        let types: Vec<Type> = config
            .types
            .iter()
            .map(|ty| ty.r#type.as_str())
            .map(Type::from)
            .collect();

        let Config { merges, .. } = config;

        for commit in revwalk
            .flatten()
            .flat_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| filter_merge_commits(commit, merges))
            .filter(|commit| filter_revert_commits(commit, self.ignore_reverts))
            .take(self.number.unwrap_or(std::usize::MAX))
        {
            total += 1;
            fail += u32::from(!print_check(&commit, &parser, &types));
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
