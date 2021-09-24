use std::{cmp::Ordering, collections::HashMap, str::FromStr};

use chrono::NaiveDate;
use git2::Time;
use regex::Regex;

use crate::{
    cli::ChangelogCommand,
    cmd::Command,
    conventional::{
        changelog::{
            ChangelogWriter, CommitContext, CommitGroup, Context, ContextBase, ContextBuilder,
            Note, NoteGroup, Reference,
        },
        config::Config,
        CommitParser, Footer,
    },
    git::{GitHelper, VersionAndTag},
    semver::SemVer,
    Error,
};

#[derive(Debug)]
struct Rev<'a>(&'a str, Option<&'a SemVer>);

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
    context_builder: ContextBuilder<'a>,
    commit_parser: CommitParser,
}

fn date_from_time(time: &Time) -> NaiveDate {
    chrono::NaiveDateTime::from_timestamp(time.seconds(), 0).date()
}
fn word_wrap_acc(mut acc: Vec<String>, word: String, line_length: usize) -> Vec<String> {
    let length = acc.len();
    if length != 0 {
        let last_line = acc.clone().pop().unwrap();
        if last_line.len() + word.len() < line_length {
            acc[length - 1] = format!("{} {}", last_line, word);
        } else {
            acc.push(word);
        }
    } else {
        acc.push(word);
    }
    acc
}

impl<'a> ChangeLogTransformer<'a> {
    fn new(config: &'a Config, git: &'a GitHelper) -> Result<Self, Error> {
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
        let commit_parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .build();

        let context_builder = ContextBuilder::new(config)?;
        Ok(Self {
            config,
            group_types,
            git,
            re_references,
            context_builder,
            commit_parser,
        })
    }

    fn make_notes(&self, footers: &'a [Footer], scope: Option<String>) -> Vec<(String, Note)> {
        let line_length = self.config.line_length;
        footers
            .iter()
            .filter(|footer| footer.key.starts_with("BREAKING"))
            .map(|footer| {
                (
                    footer.key.clone(),
                    Note {
                        scope: scope.clone(),
                        text: footer
                            .value
                            .to_owned()
                            .split_whitespace()
                            .map(|s| String::from(s))
                            .fold(Vec::<String>::new(), |acc, word| {
                                word_wrap_acc(acc, word, line_length)
                            }),
                    },
                )
            })
            .collect()
    }

    fn find_version_date(&self, spec: &str) -> Result<NaiveDate, Error> {
        let obj = self.git.repo.revparse_single(spec)?;
        Ok(
            if let Some(date) = obj
                .as_tag()
                .and_then(|tag| tag.tagger())
                .map(|tagger| tagger.when())
                .map(|time| date_from_time(&time))
            {
                date
            } else {
                let commit = obj.peel_to_commit()?;
                date_from_time(&commit.time())
            },
        )
    }

    fn transform(&self, from_rev: &Rev<'a>, to_rev: &Rev<'a>) -> Result<Context<'a>, Error> {
        let mut revwalk = self.git.revwalk()?;
        if to_rev.0.is_empty() {
            let to_commit = self.git.ref_to_commit(from_rev.0)?;
            revwalk.push(to_commit.id())?;
        } else {
            // reverse from and to as
            revwalk.push_range(format!("{}..{}", to_rev.0, from_rev.0).as_str())?;
        }
        let mut commits: HashMap<&str, Vec<CommitContext<'_>>> = HashMap::new();
        let mut notes: HashMap<String, Vec<Note>> = HashMap::new();
        let version_date = self.find_version_date(from_rev.0)?;
        for commit in revwalk
            .flatten()
            .flat_map(|oid| self.git.find_commit(oid).ok())
            .filter(|commit| commit.parent_count() <= 1)
        {
            if let Some(Ok(conv_commit)) = commit.message().map(|msg| self.commit_parser.parse(msg))
            {
                self.make_notes(&conv_commit.footers, conv_commit.scope.clone())
                    .into_iter()
                    .for_each(|(key, note)| {
                        notes.entry(key).or_default().push(note);
                    });

                let hash = commit.id().to_string();
                let date = chrono::NaiveDateTime::from_timestamp(commit.time().seconds(), 0).date();
                let scope = conv_commit.scope;
                let subject = conv_commit
                    .description
                    .to_owned()
                    .split_whitespace()
                    .map(|s| String::from(s))
                    .fold(Vec::<String>::new(), |acc, word| {
                        word_wrap_acc(acc, word, self.config.line_length)
                    });
                let body = conv_commit.body;
                let short_hash = hash[..7].into();
                let mut references = Vec::new();
                if let Some(body) = &body {
                    references.extend(self.re_references.captures_iter(body).map(|refer| {
                        Reference {
                            // TODO action (the word before?)
                            action: None,
                            owner: "",
                            repository: "",
                            prefix: refer[1].to_owned(),
                            issue: refer[2].to_owned(),
                            raw: refer[0].to_owned(),
                        }
                    }));
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
                    body,
                    short_hash,
                    references,
                };
                if let Some(section) = self.group_types.get(conv_commit.r#type.as_ref()) {
                    commits.entry(section).or_default().push(commit_context)
                }
            }
        }

        let version = if from_rev.0 == "HEAD" {
            "Unreleased"
        } else {
            from_rev.0
        };
        let is_patch = from_rev.1.map(|i| i.patch() != 0).unwrap_or(false);
        let mut commit_groups: Vec<CommitGroup<'_>> = commits
            .into_iter()
            .map(|(title, commits)| CommitGroup { title, commits })
            .collect();
        commit_groups.sort_by(|a, b| self.sort_commit_groups(a, b));
        let note_groups: Vec<NoteGroup> = notes
            .into_iter()
            .map(|(title, notes)| NoteGroup { title, notes })
            .collect();
        let Config {
            host,
            owner,
            repository,
            ..
        } = self.config;
        let context_base = ContextBase {
            version,
            date: Some(version_date),
            is_patch,
            commit_groups,
            note_groups,
            previous_tag: to_rev.0,
            current_tag: from_rev.0,
            host: host.to_owned(),
            owner: owner.to_owned(),
            repository: repository.to_owned(),
        };
        self.context_builder.build(context_base)
    }

    /// Sort commit groups based on how the configuration file contains them.
    /// The index of the first section matching the commit group title will be used as ranking.
    fn sort_commit_groups(&self, a: &CommitGroup<'_>, b: &CommitGroup<'_>) -> Ordering {
        fn find_pos(this: &ChangeLogTransformer<'_>, title: &str) -> Option<usize> {
            this.config
                .types
                .iter()
                .enumerate()
                .find(|(_, x)| x.section == title)
                .map(|(i, _)| i)
        }
        let pos_a = find_pos(self, a.title);
        let pos_b = find_pos(self, b.title);
        pos_a.cmp(&pos_b)
    }
}

impl Command for ChangelogCommand {
    fn exec(&self, config: Config) -> Result<(), Error> {
        let helper = GitHelper::new(self.prefix.as_str())?;
        let rev = self.rev.as_str();
        let (rev, rev_stop) = if rev.contains("..") {
            let mut split = rev.split("..");
            let low = split.next().unwrap_or("");
            let hi = split
                .next()
                .map(|s| if s.is_empty() { "HEAD" } else { s })
                .unwrap_or("HEAD");
            // FIXME hi and low are supposed to be a version tag.
            (hi, low)
        } else {
            (rev, "")
        };

        let stdout = std::io::stdout();
        let stdout = stdout.lock();
        let template = config.template.as_deref();
        let mut writer = ChangelogWriter::new(template, stdout)?;
        writer.write_header(config.header.as_str())?;

        let transformer = ChangeLogTransformer::new(&config, &helper)?;
        match helper.find_last_version(rev)? {
            Some(last_version) => {
                let semver = SemVer::from_str(rev.trim_start_matches(&self.prefix));
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
                    .chain(Some(Rev(rev_stop, None)));
                let iter: Vec<Rev<'_>> = iter.collect();
                for w in iter.windows(2) {
                    let from = &w[0];
                    let to = &w[1];
                    let context = transformer.transform(from, to)?;
                    if !self.skip_empty || !context.context.commit_groups.is_empty() {
                        writer.write_template(&context)?;
                    }
                }
            }
            None => {
                let context = transformer.transform(&Rev("HEAD", None), &Rev("", None))?;
                if !self.skip_empty || !context.context.commit_groups.is_empty() {
                    writer.write_template(&context)?;
                }
            }
        }
        Ok(())
    }
}
