use crate::{cli::CheckCommand, cmd::Command, conventional, Error};
use git2::{Commit, Repository};

fn print_check(commit: &Commit<'_>) -> bool {
    let msg = std::str::from_utf8(commit.message_bytes()).expect("valid utf-8 message");
    let short_id = commit.as_object().short_id().unwrap();
    let short_id = short_id.as_str().expect("short id");
    let msg_parsed: Result<conventional::Commit, &str> = msg.parse();
    match msg_parsed {
        Err(e) => {
            let first_line = msg.lines().next().unwrap_or("");
            let short_msg: String = first_line.chars().take(40).collect();
            if first_line.len() > 40 {
                println!("FAIL   {}   {}   {}...", short_id, e, short_msg)
            } else {
                println!("FAIL   {}   {}   {}", short_id, e, short_msg)
            }
            false
        }
        _ => true,
    }
}

impl Command for CheckCommand {
    fn exec(&self) -> Result<(), Error> {
        let repo = Repository::open_from_env()?;
        let mut revwalk = repo.revwalk()?;
        if self.rev.contains("..") {
            revwalk.push_range(self.rev.as_str())?;
        } else {
            revwalk.push_ref(self.rev.as_str())?;
        }

        let mut total = 0;
        let mut fail = 0;

        for commit in revwalk
            .flatten()
            .flat_map(|oid| repo.find_commit(oid).ok())
            .filter(|commit| commit.parent_count() <= 1)
        {
            total += 1;
            fail += u32::from(!print_check(&commit));
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
            Err(Error::Check)
        }
    }
}
