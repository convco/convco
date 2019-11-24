use crate::{
    cli::ChangelogCommand,
    cmd::Command,
    conventional::changelog::{
        ChangelogWriter, CommitContext, CommitGroup, Config, ContextBuilder,
    },
    git::{GitHelper, VersionAndTag},
    Error,
};

use semver::Version;
use std::{collections::HashMap, io, str::FromStr};

#[derive(Debug)]
struct Rev<'a>(&'a str, Option<&'a Version>);

impl<'a> From<&'a VersionAndTag> for Rev<'a> {
    fn from(tav: &'a VersionAndTag) -> Self {
        Rev(tav.tag.as_str(), Some(&tav.version))
    }
}

impl ChangelogCommand {
    fn make_changelog_for(
        &self,
        config: &Config,
        writer: &mut ChangelogWriter<impl io::Write>,
        helper: &GitHelper,
        from_rev: &Rev<'_>,
        to_rev: &Rev<'_>,
    ) -> Result<(), Error> {
        // TODO: this should be a parameter
        let group_types =
            config
                .types
                .iter()
                .filter(|ty| !ty.hidden)
                .fold(HashMap::new(), |mut acc, ty| {
                    acc.insert(ty.r#type.as_str(), ty.section.as_str());
                    acc
                });
        let mut revwalk = helper.revwalk()?;
        if to_rev.0 == "" {
            let to_commit = helper.ref_to_commit(from_rev.0)?;
            revwalk.push(to_commit.id())?;
        } else {
            // reverse from and to as
            revwalk.push_range(format!("{}..{}", to_rev.0, from_rev.0).as_str())?;
        }
        let mut commits: HashMap<&str, Vec<CommitContext>> = HashMap::new();
        let mut version_date = None;
        for commit in revwalk
            .flatten()
            .flat_map(|oid| helper.find_commit(oid).ok())
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
                if let Some(section) = group_types.get(conv_commit.r#type.as_ref()) {
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

        let mut builder = ContextBuilder::new(&config)?
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

        let context = builder.build()?;
        writer.write_template(&context)?;
        Ok(())
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

        match helper.find_last_version(rev)? {
            Some(v) => {
                let mut versions = helper.versions_from(&v);
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
                let to_rev = versions.pop();
                match to_rev {
                    None => self.make_changelog_for(
                        &config,
                        &mut writer,
                        &helper,
                        &from_rev,
                        &Rev("", None),
                    )?,
                    Some(tav) => {
                        let mut rev = tav.into();
                        self.make_changelog_for(&config, &mut writer, &helper, &from_rev, &rev)?;
                        loop {
                            let from_rev = rev;
                            match versions.pop() {
                                None => {
                                    self.make_changelog_for(
                                        &config,
                                        &mut writer,
                                        &helper,
                                        &from_rev,
                                        &Rev("", None),
                                    )?;
                                    break;
                                }
                                Some(tav) => {
                                    rev = tav.into();
                                    self.make_changelog_for(
                                        &config,
                                        &mut writer,
                                        &helper,
                                        &from_rev,
                                        &rev,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            None => self.make_changelog_for(
                &config,
                &mut writer,
                &helper,
                &Rev("HEAD", None),
                &Rev("", None),
            )?,
        }
        Ok(())
    }
}
