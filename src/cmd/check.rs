use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt,
    io::{stdin, Read},
};

use convco::{
    open_repo, strip::Strip, Commit, CommitParser, CommitTrait, Config, ConvcoError, Repo,
    RevWalkOptions,
};
use jiff::Zoned;

use crate::{cli::CheckCommand, cmd::Command};

fn print_fail(msg: Cow<str>, short_id: &str, e: impl fmt::Display) -> bool {
    let first_line = msg.lines().next().unwrap_or("");
    let short_msg: String = first_line.chars().take(40).collect();
    if first_line.len() > 40 {
        println!("FAIL  {}  {}  {}...", short_id, e, short_msg)
    } else {
        println!("FAIL  {}  {}  {}", short_id, e, short_msg)
    }
    false
}

struct TypeErrorWithSimilaritySuggestions<'a, 'b> {
    valid_types: &'a [String],
    wrong_type: &'b str,
}

impl fmt::Display for TypeErrorWithSimilaritySuggestions<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            valid_types,
            wrong_type,
        } = self;

        f.write_fmt(format_args!("wrong type: {wrong_type}"))?;
        if let Some((suggestion, _)) = valid_types
            .iter()
            .map(|s| (s, strsim::jaro_winkler(wrong_type, s)))
            .min_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Ordering::Equal))
        {
            f.write_fmt(format_args!(", did you mean `{suggestion}`"))?;
        }

        Ok(())
    }
}

fn print_wrong_type(
    msg: Cow<str>,
    short_id: &str,
    commit_type: String,
    valid_types: &[String],
) -> bool {
    print_fail(
        msg,
        short_id,
        TypeErrorWithSimilaritySuggestions {
            wrong_type: &commit_type,
            valid_types,
        },
    )
}

fn print_check<O: CommitTrait>(
    commit: Result<Commit<O>, (ConvcoError, O)>,
    types: &[String],
) -> bool {
    match commit {
        Err((e, o)) => print_fail(o.commit_message().unwrap(), &o.short_id(), e),
        Ok(Commit {
            conventional_commit,
            commit: oid,
        }) if !types.contains(&conventional_commit.r#type) => print_wrong_type(
            conventional_commit.description.into(),
            &oid.short_id(),
            conventional_commit.r#type,
            types,
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

        let mut total = 0;
        let mut fail = 0;

        let parser = CommitParser::builder()
            .scope_regex(config.scope_regex)
            .strip_regex(config.strip_regex)
            .build();
        let types: Vec<String> = config
            .types
            .iter()
            .map(|ty| ty.r#type.as_str())
            .map(String::from)
            .collect();

        if self.from_stdin {
            #[derive(Debug, Clone)]
            struct CommitDummy(String);
            impl convco::CommitTrait for CommitDummy {
                type ObjectId = String;

                fn short_id(&self) -> String {
                    "-".to_owned()
                }

                fn commit_message(&self) -> Result<Cow<'_, str>, ConvcoError> {
                    Ok(Cow::Borrowed(&self.0))
                }

                fn id(&self) -> String {
                    self.short_id()
                }

                fn oid(&self) -> Self::ObjectId {
                    self.short_id()
                }

                fn commit_time(&self) -> Result<jiff::Zoned, ConvcoError> {
                    Ok(Zoned::now())
                }
            }
            let mut stdin = stdin().lock();
            let mut commit_msg = String::new();
            stdin.read_to_string(&mut commit_msg)?;
            if self.strip {
                commit_msg = commit_msg.strip();
            }
            let commit = CommitDummy(commit_msg);
            let result = match parser.parse(&commit.0) {
                Ok(conventional_commit) => Ok(Commit {
                    conventional_commit,
                    commit,
                }),
                Err(e) => Err((e.into(), commit)),
            };

            let is_conventional = print_check(result, &types);
            match is_conventional {
                true => return Ok(()),
                false => return Err(ConvcoError::Check)?,
            }
        }

        let repo = open_repo()?;
        let (to_rev, from_rev) = match self.rev.as_ref() {
            Some(rev) => match rev.split_once("..") {
                None => {
                    let rev = Repo::revparse_single(&repo, rev)?;
                    (rev, None)
                }
                Some(("", rev)) => {
                    let rev = Repo::revparse_single(&repo, rev)?;
                    (rev, None)
                }
                Some((rev_stop, "")) => {
                    let rev = Repo::revparse_single(&repo, "HEAD")?;
                    let rev_stop = Repo::revparse_single(&repo, rev_stop)?;
                    (rev, Some(rev_stop))
                }
                Some((rev, rev_stop)) => {
                    let rev = Repo::revparse_single(&repo, rev)?;
                    let rev_stop = Repo::revparse_single(&repo, rev_stop)?;
                    (rev, Some(rev_stop))
                }
            },

            None => (Repo::revparse_single(&repo, "HEAD")?, None),
        };
        let options = RevWalkOptions {
            from_rev: from_rev.into_iter().collect(),
            to_rev,
            first_parent: config.first_parent,
            no_merge_commits: !config.merges,
            no_revert_commits: self.ignore_reverts,
            paths: vec![],
            parser: &parser,
        };
        let revwalk = Repo::revwalk(&repo, options)?;

        for commit in revwalk.take(self.number.unwrap_or(usize::MAX)) {
            total += 1;
            fail += u32::from(!print_check(commit, &types));
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
            Err(ConvcoError::Check)?
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_suggestions() {
        let output = super::TypeErrorWithSimilaritySuggestions {
            wrong_type: "tests",
            valid_types: &[
                "feat", "fix", "build", "chore", "ci", "docs", "style", "refactor", "perf", "test",
            ]
            .map(|s| s.to_string()),
        }
        .to_string();

        assert_eq!(output, "wrong type: tests, did you mean `test`");
    }
}
