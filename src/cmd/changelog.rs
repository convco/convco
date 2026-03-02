use std::{cmp::Ordering, collections::HashMap, io::Write, path::PathBuf, str::FromStr};

use anyhow::Context as _;
use git2::Time;
use jiff::{
    civil::Date,
    tz::{Offset, TimeZone},
    Timestamp,
};

use crate::{
    cli::ChangelogCommand,
    cmd::Command,
    conventional::{
        changelog::{
            ChangelogWriter, CommitContext, CommitGroup, Context, ContextBase, ContextBuilder,
            Note, NoteGroup, Reference,
        },
        config::Config,
        CommitParser, Footer, FooterKey,
    },
    git::{filter_merge_commits, GitHelper, VersionAndTag},
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

struct Unreleased {
    str: String,
    version: Option<SemVer>,
}

/// Transforms a range of commits to pass them to the changelog writer.
struct ChangeLogTransformer<'a> {
    group_types: HashMap<&'a str, &'a str>,
    config: &'a Config,
    unreleased: Unreleased,
    git: &'a GitHelper,
    context_builder: ContextBuilder<'a>,
    commit_parser: CommitParser,
    paths: &'a [PathBuf],
    ignore_paths: &'a [PathBuf],
    prefix: &'a str,
}

fn date_from_time(time: &Time) -> Date {
    Timestamp::from_second(time.seconds())
        .unwrap()
        .to_zoned(TimeZone::fixed(
            Offset::from_seconds(time.offset_minutes() * 60).unwrap(),
        ))
        .date()
}

impl<'a> ChangeLogTransformer<'a> {
    fn new(
        config: &'a Config,
        include_hidden_sections: bool,
        git: &'a GitHelper,
        paths: &'a [PathBuf],
        ignore_paths: &'a [PathBuf],
        unreleased: String,
        prefix: &'a str,
    ) -> Result<Self, Error> {
        let group_types = config
            .types
            .iter()
            .filter(|ty| include_hidden_sections || !ty.hidden)
            .fold(HashMap::new(), |mut acc, ty| {
                acc.insert(ty.r#type.as_str(), ty.section.as_str());
                acc
            });
        let commit_parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .strip_regex(config.strip_regex.clone())
            .references_regex(format!("({})([0-9]+)", config.issue_prefixes.join("|")))
            .build();

        let context_builder = ContextBuilder::new(config)?;
        let unreleased = match unreleased.parse::<SemVer>() {
            Ok(version) => Unreleased {
                str: unreleased,
                version: Some(version),
            },
            _ => Unreleased {
                str: unreleased,
                version: None,
            },
        };

        Ok(Self {
            config,
            group_types,
            git,
            context_builder,
            commit_parser,
            paths,
            ignore_paths,
            unreleased,
            prefix,
        })
    }

    fn make_notes(&self, footers: &'a [Footer], scope: Option<String>) -> Vec<(String, Note)> {
        footers
            .iter()
            .filter(|footer| matches!(footer.key, FooterKey::BreakingChange))
            .map(|footer| {
                (
                    footer.key.to_string(),
                    Note {
                        scope: scope.clone(),
                        text: footer.value.clone(),
                    },
                )
            })
            .collect()
    }

    fn find_version_date(&self, spec: &str) -> Result<Date, Error> {
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

    fn transform(&'a self, from_rev: &Rev<'a>, to_rev: &Rev<'a>) -> Result<Context<'a>, Error> {
        let mut revwalk = self.git.revwalk()?;
        if self.config.first_parent {
            revwalk.simplify_first_parent()?;
        }
        if to_rev.0.is_empty() {
            let to_commit = self.git.ref_to_commit(from_rev.0)?;
            revwalk.push(to_commit.id())?;
        } else {
            // reverse from and to as
            revwalk.push_range(format!("{}..{}", to_rev.0, from_rev.0).as_str())?;
        }
        let mut commits: HashMap<&str, Vec<CommitContext>> = HashMap::new();
        let mut notes: HashMap<String, Vec<Note>> = HashMap::new();
        let version_date = self.find_version_date(from_rev.0)?;
        let Config {
            host,
            owner,
            repository,
            merges,
            ..
        } = self.config;
        for commit in revwalk
            .flatten()
            .flat_map(|oid| self.git.find_commit(oid).ok())
            .filter(|commit| {
                self.git
                    .commit_updates_relevant_paths(commit, self.paths, self.ignore_paths)
            })
            .filter(|commit| filter_merge_commits(commit, *merges))
        {
            if let Some(Ok(conv_commit)) = commit.message().map(|msg| self.commit_parser.parse(msg))
            {
                self.make_notes(&conv_commit.footers, conv_commit.scope.clone())
                    .into_iter()
                    .for_each(|(key, note)| {
                        notes.entry(key).or_default().push(note);
                    });

                let hash = commit.id().to_string();
                let date = date_from_time(&commit.time());
                let scope = conv_commit.scope;
                let subject = conv_commit.description;
                let body = conv_commit.body;
                let short_hash = hash[..7].into();
                let references = conv_commit
                    .references
                    .into_iter()
                    .map(|r| Reference {
                        action: r.action,
                        owner: owner.as_deref().unwrap_or_default(),
                        repository: repository.as_deref().unwrap_or_default(),
                        prefix: r.prefix,
                        issue: r.issue,
                    })
                    .collect();
                let commit_context = CommitContext {
                    hash,
                    date,
                    scope,
                    subject,
                    body,
                    short_hash,
                    references,
                };
                if let Some(section) = self.group_types.get(conv_commit.r#type.as_str()) {
                    commits.entry(section).or_default().push(commit_context)
                }
            }
        }

        let version = if from_rev.0 == "HEAD" {
            match &self.unreleased.version {
                Some(v) => format!("{}{}", self.prefix, v.0).into(),
                None => self.unreleased.str.as_str().into(),
            }
        } else {
            from_rev.0.into()
        };
        let is_patch = from_rev.1.map(|i| i.patch() != 0).unwrap_or(false)
            || (self.unreleased.str == version
                && self
                    .unreleased
                    .version
                    .as_ref()
                    .map(|i| i.patch() != 0)
                    .unwrap_or(false));
        let mut commit_groups: Vec<CommitGroup<'_>> = commits
            .into_iter()
            .map(|(title, commits)| CommitGroup { title, commits })
            .collect();
        commit_groups.sort_by(|a, b| self.sort_commit_groups(a, b));
        let note_groups: Vec<NoteGroup> = notes
            .into_iter()
            .map(|(title, notes)| NoteGroup { title, notes })
            .collect();

        let current_tag = if from_rev.0 == "HEAD" {
            let id = self.git.ref_to_commit("HEAD")?.id();

            id.to_string()
        } else {
            from_rev.0.to_owned()
        };

        let context_base = ContextBase {
            version,
            date: Some(version_date),
            is_patch,
            commit_groups,
            note_groups,
            previous_tag: to_rev.0,
            current_tag: current_tag.into(),
            host: host.to_owned(),
            owner: owner.to_owned(),
            repository: repository.to_owned(),
            link_compare: self.config.link_compare,
            link_references: self.config.link_references,
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

impl ChangelogCommand {
    pub(crate) fn write(&self, mut config: Config, stdout: impl Write) -> anyhow::Result<()> {
        let ignore_paths = if self.ignore_paths.is_empty() {
            config.ignore_paths.clone()
        } else {
            self.ignore_paths.clone()
        };
        if self.no_links {
            config.link_references = false;
            config.link_compare = false;
        }
        if self.merges {
            config.merges = true;
        }
        if self.first_parent {
            config.first_parent = true;
        }
        if let Some(line_length) = self.line_length {
            config.line_length = line_length;
        }
        if self.no_wrap {
            config.wrap_disabled = true;
        }

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

        let template = config.template.as_deref();
        let mut writer = ChangelogWriter::new(template, &config, stdout)?;
        writer.write_header(config.header.as_str())?;

        let transformer = ChangeLogTransformer::new(
            &config,
            self.include_hidden_sections,
            &helper,
            &self.paths,
            &ignore_paths,
            self.unreleased.clone(),
            &self.prefix,
        )?;
        match helper
            .find_last_version(rev, self.ignore_prereleases)
            .with_context(|| format!("Could not find the last version for revision {rev}"))?
        {
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
                    iter.chain(Some(Rev(
                        last_version.tag.as_str(),
                        Some(&last_version.version),
                    )))
                } else {
                    iter.chain(None)
                };

                let stop_at_major =
                    (last_version.version.0.major + 1).saturating_sub(self.max_majors);
                let stop_at_minor =
                    (last_version.version.0.minor + 1).saturating_sub(self.max_minors);
                let stop_at_patch =
                    (last_version.version.0.patch + 1).saturating_sub(self.max_patches);

                let iter = iter
                    .chain(
                        helper
                            .versions_from(&last_version)
                            .into_iter()
                            .rev()
                            .take_while(|v| v.tag != rev_stop)
                            .map(|v| v.into()),
                    )
                    .chain(Some(Rev(rev_stop, None)))
                    .take(self.max_versions.map(|x| x + 1).unwrap_or(std::usize::MAX));
                let iter: Vec<Rev<'_>> = iter.collect();
                for w in iter.windows(2) {
                    let from = match &w[0] {
                        Rev(_, Some(v))
                            if v.major() < stop_at_major
                                || v.minor() < stop_at_minor
                                || v.patch() < stop_at_patch =>
                        {
                            break
                        }
                        _ => &w[0],
                    };
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

impl Command for ChangelogCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let out: Box<dyn Write> = match self.output.as_path() {
            p if p.to_string_lossy() == "-" => Box::new(std::io::stdout().lock()),
            p => Box::new(std::fs::File::create(p)?),
        };
        self.write(config, out)?;
        Ok(())
    }
}
