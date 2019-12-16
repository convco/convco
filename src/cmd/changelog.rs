use crate::{
    cli::ChangelogCommand,
    cmd::Command,
    conventional::{
        changelog::{ChangelogWriter, CommitContext, CommitGroup, Config, ContextBuilder},
        Footer,
    },
    git::{GitHelper, VersionAndTag},
    Error,
};

use crate::conventional::changelog::{Context, Note, NoteGroup, Reference};
use regex::Regex;
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
    re_references: Regex,
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
        let re_references =
            Regex::new(format!("({})([0-9]+)", config.issue_prefixes.join("|")).as_str()).unwrap();
        Self {
            config,
            group_types,
            git,
            re_references,
        }
    }

    fn make_notes(&self, footers: &'a [Footer], scope: Option<String>) -> Vec<(String, Note)> {
        footers
            .iter()
            .filter(|footer| footer.key.starts_with("BREAKING"))
            .map(|footer| {
                (
                    footer.key.clone(),
                    Note {
                        scope: scope.clone(),
                        text: footer.value.clone(),
                    },
                )
            })
            .collect()
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
        let mut notes: HashMap<String, Vec<Note>> = HashMap::new();
        let mut version_date = None;
        for commit in revwalk
            .flatten()
            .flat_map(|oid| self.git.find_commit(oid).ok())
            .filter(|commit| commit.parent_count() <= 1)
        {
            if let Some(Ok(conv_commit)) =
                commit.message().map(crate::conventional::Commit::from_str)
            {
                self.make_notes(&conv_commit.footers, conv_commit.scope.clone())
                    .into_iter()
                    .for_each(|(key, note)| {
                        notes.entry(key).or_default().push(note);
                    });

                let hash = commit.id().to_string();
                let date = chrono::NaiveDateTime::from_timestamp(commit.time().seconds(), 0).date();
                let scope = conv_commit.scope;
                let subject = conv_commit.description;
                let short_hash = hash[..7].into();
                let mut references = Vec::new();
                if let Some(body) = conv_commit.body {
                    references.extend(self.re_references.captures_iter(body.as_str()).map(
                        |refer| Reference {
                            // TODO action (the word before?)
                            action: None,
                            owner: "",
                            repository: "",
                            prefix: refer[1].to_owned(),
                            issue: refer[2].to_owned(),
                            raw: refer[0].to_owned(),
                        },
                    ));
                }
                references.extend(conv_commit.footers.iter().flat_map(|footer| {
                    self.re_references
                        .captures_iter(footer.value.as_str())
                        .map(move |refer| Reference {
                            action: Some(footer.key.clone()),
                            owner: "",
                            repository: "",
                            prefix: refer[1].to_owned(),
                            issue: refer[2].to_owned(),
                            raw: refer[0].to_owned(),
                        })
                }));
                let commit_context = CommitContext {
                    hash,
                    date,
                    scope,
                    subject,
                    short_hash,
                    references,
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
            .previous_tag(to_rev.0)
            .current_tag(from_rev.0)
            .commit_groups(
                commits
                    .into_iter()
                    .map(|(title, commits)| CommitGroup { title, commits })
                    .collect(),
            )
            .note_groups(
                notes
                    .into_iter()
                    .map(|(title, notes)| NoteGroup { title, notes })
                    .collect(),
            );

        if let Some(date) = version_date {
            builder = builder.date(date);
        }

        Ok(builder.build()?)
    }
}

fn make_cl_config() -> Config {
    std::fs::read(".versionrc")
        .ok()
        .and_then(|versionrc| serde_yaml::from_reader(versionrc.as_slice()).ok())
        .unwrap_or_default()
}

impl Command for ChangelogCommand {
    fn exec(&self) -> Result<(), Error> {
        let helper = GitHelper::new(self.prefix.as_str())?;

        let rev = self.rev.as_str();
        let (rev, rev_stop) = if rev.contains("..") {
            let mut split = rev.split("..");
            let low = split.next().unwrap_or("");
            let hi = split
                .next()
                .map(|s| if s == "" { "HEAD" } else { s })
                .unwrap_or("HEAD");
            // FIXME hi and low are supposed to be a version tag.
            (hi, low)
        } else {
            (rev, "")
        };

        let config = make_cl_config();
        let stdout = std::io::stdout();
        let stdout = stdout.lock();
        let mut writer = ChangelogWriter { writer: stdout };
        writer.write_header(config.header.as_str())?;

        let transformer = ChangeLogTransformer::new(&config, &helper);

        match helper.find_last_version(rev)? {
            Some(last_version) => {
                let semver = Version::from_str(rev.trim_start_matches(&self.prefix));
                let from_rev = if let Ok(ref semver) = &semver {
                    if helper.same_commit(rev, last_version.tag.as_str()) {
                        Rev(last_version.tag.as_str(), Some(semver))
                    } else {
                        Rev(rev, Some(semver))
                    }
                } else if helper.same_commit(rev, last_version.tag.as_str()) {
                    Rev(last_version.tag.as_str(), Some(&last_version.version))
                } else {
                    Rev(rev, None)
                };
                // TODO refactor this logic a bit to be less complicated.
                //  if we have HEAD!=version tag - version tag - ...
                //  or HEAD==version tag - version tag - ...
                //  we have to use different logic here, or in the `GitHelper::versions_from` method.
                let is_head = from_rev.0 == "HEAD";
                let iter = Some(from_rev).into_iter();
                let iter = if is_head {
                    iter.chain(
                        Some(Rev(last_version.tag.as_str(), Some(&last_version.version)))
                            .into_iter(),
                    )
                } else {
                    iter.chain(None)
                };
                let iter = iter
                    .chain(
                        helper
                            .versions_from(&last_version)
                            .into_iter()
                            .rev()
                            .take_while(|v| v.tag != rev_stop)
                            .map(|v| v.into()),
                    )
                    .chain(Some(Rev("", None)));
                let iter: Vec<Rev<'_>> = iter.collect();
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
