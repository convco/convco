use crate::{
    cli::ChangelogCommand,
    cmd::Command,
    conventional::changelog::{
        ChangelogWriter, CommitContext, CommitGroup, Config, ContextBuilder,
    },
    git::{GitHelper, VersionAndTag},
    Error,
};

use crate::conventional::changelog::Context;
use semver::Version;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug)]
struct Rev<'a>(&'a str, Option<&'a Version>);

impl<'a> From<&'a VersionAndTag> for Rev<'a> {
    fn from(tav: &'a VersionAndTag) -> Self {
        Rev(tav.tag.as_str(), Some(&tav.version))
    }
}

/// Transforms a range of commits to pass them to the changelog writer.
struct ChangeLogTransformer<'a> {
    group_types: HashMap<&'a str, &'a str>,
    config: &'a Config,
    git: &'a GitHelper,
}

impl<'a> ChangeLogTransformer<'a> {
    fn new(config: &'a Config, git: &'a GitHelper) -> Self {
        let group_types =
            config
                .types
                .iter()
                .filter(|ty| !ty.hidden)
                .fold(HashMap::new(), |mut acc, ty| {
                    acc.insert(ty.r#type.as_str(), ty.section.as_str());
                    acc
                });
        Self {
            config,
            group_types,
            git,
        }
    }

    fn transform(&self, from_rev: &Rev<'a>, to_rev: &Rev<'a>) -> Result<Context<'a>, Error> {
        let mut revwalk = self.git.revwalk()?;
        if to_rev.0 == "" {
            let to_commit = self.git.ref_to_commit(from_rev.0)?;
            revwalk.push(to_commit.id())?;
        } else {
            // reverse from and to as
            revwalk.push_range(format!("{}..{}", to_rev.0, from_rev.0).as_str())?;
        }
        let mut commits: HashMap<&str, Vec<CommitContext>> = HashMap::new();
        let mut version_date = None;
        for commit in revwalk
            .flatten()
            .flat_map(|oid| self.git.find_commit(oid).ok())
            .filter(|commit| commit.parent_count() <= 1)
        {
            if let Some(Ok(conv_commit)) =
                commit.message().map(crate::conventional::Commit::from_str)
            {
                let hash = commit.id().to_string();
                let date = chrono::NaiveDateTime::from_timestamp(commit.time().seconds(), 0).date();
                let subject = conv_commit.description;
                let short_hash = hash[..7].into();
                let commit_context = CommitContext {
                    hash,
                    date,
                    subject,
                    short_hash,
                };
                if let Some(section) = self.group_types.get(conv_commit.r#type.as_ref()) {
                    if version_date.is_none() {
                        version_date = Some(date);
                    }
                    commits.entry(section).or_default().push(commit_context)
                }
            }
        }

        let version = if from_rev.0 == "HEAD" {
            "Unreleased"
        } else {
            from_rev.0
        };
        let is_patch = from_rev.1.map(|i| i.patch != 0).unwrap_or(false);

        let mut builder = ContextBuilder::new(self.config)?
            .version(version)
            .is_patch(is_patch)
            .commit_groups(
                commits
                    .into_iter()
                    .map(|(title, commits)| CommitGroup { title, commits })
                    .collect(),
            );

        if let Some(date) = version_date {
            builder = builder.date(date);
        }

        Ok(builder.build()?)
    }
}

fn make_cl_config() -> Config {
    Config::default()
}

impl Command for ChangelogCommand {
    fn exec(&self) -> Result<(), Error> {
        let helper = GitHelper::new(self.prefix.as_str())?;
        let rev = self.rev.as_str();

        let config = make_cl_config();
        let stdout = std::io::stdout();
        let stdout = stdout.lock();
        let mut writer = ChangelogWriter { writer: stdout };
        writer.write_header(config.header.as_str())?;

        let transformer = ChangeLogTransformer::new(&config, &helper);

        match helper.find_last_version(rev)? {
            Some(v) => {
                let semver = Version::from_str(rev.trim_start_matches(&self.prefix));
                let from_rev = if let Ok(ref semver) = &semver {
                    if helper.same_commit(rev, v.tag.as_str()) {
                        Rev(v.tag.as_str(), Some(semver))
                    } else {
                        Rev(rev, Some(semver))
                    }
                } else if helper.same_commit(rev, v.tag.as_str()) {
                    Rev(v.tag.as_str(), Some(&v.version))
                } else {
                    Rev(rev, None)
                };

                let iter: Vec<Rev<'_>> = Some(from_rev)
                    .into_iter()
                    .chain(helper.versions_from(&v).into_iter().rev().map(|v| v.into()))
                    .chain(Some(Rev("", None)))
                    .collect();
                for w in iter.windows(2) {
                    let from = &w[0];
                    let to = &w[1];
                    let context = transformer.transform(from, to)?;
                    writer.write_template(&context)?;
                }
            }
            None => {
                let context = transformer.transform(&Rev("HEAD", None), &Rev("", None))?;
                writer.write_template(&context)?;
            }
        }
        Ok(())
    }
}
