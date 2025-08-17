use std::{borrow::Cow, cmp::Ordering, collections::HashMap, io::Write};

use anyhow::Context as _;
use convco::{
    changelog::{
        ChangelogWriter, CommitContext, CommitGroup, Context, ContextBase, ContextBuilder, Note,
        NoteGroup, Reference,
    },
    open_repo, CommitParser, CommitTrait, Config, ConvcoError, Footer, FooterKey, MaxMajorsIterExt,
    MaxMinorsIterExt, MaxPatchesIterExt, Repo, RevWalkOptions,
};
use semver::Version;

use crate::{cli::ChangelogCommand, Command};

#[derive(Debug, Clone)]
struct Rev<C>(Option<C>, Option<Version>);

struct Unreleased {
    str: String,
    version: Option<Version>,
}

/// Transforms a range of commits to pass them to the changelog writer.
struct ChangeLogTransformer<'a, R: Repo<'a>> {
    group_types: HashMap<&'a str, &'a str>,
    config: &'a Config,
    revwalk_options: RevWalkOptions<'a, R::CommitTrait>,
    unreleased: Unreleased,
    repo: &'a R,
    context_builder: ContextBuilder<'a>,
    prefix: &'a str,
}

impl<'a, R: Repo<'a>> ChangeLogTransformer<'a, R> {
    fn new(
        config: &'a Config,
        include_hidden_sections: bool,
        repo: &'a R,
        revwalk_options: RevWalkOptions<'a, R::CommitTrait>,
        unreleased: String,
        prefix: &'a str,
    ) -> Result<Self, ConvcoError> {
        let group_types = config
            .types
            .iter()
            .filter(|ty| include_hidden_sections || !ty.hidden)
            .fold(HashMap::new(), |mut acc, ty| {
                acc.insert(ty.r#type.as_str(), ty.section.as_str());
                acc
            });

        let context_builder = ContextBuilder::new(config)?;
        let unreleased = match unreleased.parse::<Version>() {
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
            repo,
            revwalk_options,
            context_builder,
            unreleased,
            prefix,
        })
    }

    fn make_notes(&self, footers: &'_ [Footer], scope: Option<String>) -> Vec<(String, Note)> {
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

    fn transform(
        &self,
        to_rev: Rev<R::CommitTrait>,
        from_rev: Rev<R::CommitTrait>,
    ) -> Result<Context<'_>, ConvcoError> {
        let revwalk_options = RevWalkOptions {
            from_rev: {
                let mut rev = self.revwalk_options.from_rev.clone();
                if let Some(from_rev) = from_rev.0.as_ref() {
                    rev.push(from_rev.clone());
                }
                rev
            },
            to_rev: to_rev.0.as_ref().unwrap().clone(),
            ..self.revwalk_options.clone()
        };

        let revwalk = self.repo.revwalk(revwalk_options)?;
        let mut commits: HashMap<&str, Vec<CommitContext>> = HashMap::new();
        let mut notes: HashMap<String, Vec<Note>> = HashMap::new();
        let version_date = to_rev
            .0
            .as_ref()
            .and_then(|c| c.commit_time().ok())
            .unwrap()
            .date();
        let Config {
            host,
            owner,
            repository,
            ..
        } = self.config;
        for commit in revwalk.flatten() {
            let conv_commit = commit.conventional_commit;
            let footers = conv_commit.footers;
            self.make_notes(&footers, conv_commit.scope.clone())
                .into_iter()
                .for_each(|(key, note)| {
                    notes.entry(key).or_default().push(note);
                });

            let hash = commit.commit.id();
            let date = commit.commit.commit_time()?.date();
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

        let version: Cow<str> = if to_rev.1.is_none() {
            match &self.unreleased.version {
                Some(v) => format!("{}{}", self.prefix, v).into(),
                None => self.unreleased.str.as_str().into(),
            }
        } else {
            format!("{}{}", self.prefix, to_rev.1.as_ref().unwrap()).into()
        };
        let is_patch = from_rev.1.as_ref().map(|i| i.patch != 0).unwrap_or(false)
            || (self.unreleased.str == version
                && self
                    .unreleased
                    .version
                    .as_ref()
                    .map(|i| i.patch != 0)
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

        let current_tag = to_rev
            .1
            .as_ref()
            .map(|v| v.to_string().into())
            .unwrap_or("".into());
        let previous_tag = match to_rev.1.as_ref() {
            Some(version) => format!("{}{}", self.prefix, version),
            None => "".into(),
        };
        let context_base = ContextBase {
            version,
            date: Some(version_date),
            is_patch,
            commit_groups,
            note_groups,
            previous_tag,
            current_tag,
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
        fn find_pos<'a, R: Repo<'a>>(
            this: &ChangeLogTransformer<'a, R>,
            title: &str,
        ) -> Option<usize> {
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
        let repo = open_repo()?;

        let rev_str = self.rev.as_str();
        let (rev_high, rev_low) = match rev_str.split_once("..") {
            None => {
                let rev_high = Repo::revparse_single(&repo, rev_str)?;
                (rev_high, None)
            }
            Some(("", rev)) => {
                let rev_high = Repo::revparse_single(&repo, rev)?;
                (rev_high, None)
            }
            Some((rev_low, "")) => {
                let rev_high = Repo::revparse_single(&repo, "HEAD")?;
                let rev_low = Repo::revparse_single(&repo, rev_low)?;
                (rev_high, Some(rev_low))
            }
            Some((rev_low, rev_high)) => {
                let rev_high = Repo::revparse_single(&repo, rev_high)?;
                let rev_low = Repo::revparse_single(&repo, rev_low)?;
                (rev_high, Some(rev_low))
            }
        };
        let template = config.template.as_deref();
        let mut writer = ChangelogWriter::new(template, &config, stdout)?;
        writer.write_header(config.header.as_str())?;
        let commit_parser = CommitParser::builder()
            .scope_regex(config.scope_regex.clone())
            .strip_regex(config.strip_regex.clone())
            .references_regex(format!("({})([0-9]+)", config.issue_prefixes.join("|")))
            .build();
        let revwalk_options = RevWalkOptions {
            from_rev: rev_low.iter().cloned().collect(),
            to_rev: rev_high.clone(),
            first_parent: config.first_parent,
            no_merge_commits: !config.merges,
            no_revert_commits: false, // FIXME no_revert_commits,
            paths: self
                .paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            parser: &commit_parser,
        };
        let transformer = ChangeLogTransformer::new(
            &config,
            self.include_hidden_sections,
            &repo,
            revwalk_options,
            self.unreleased.clone(),
            &self.prefix,
        )?;
        let semvers = repo.semver_tags(&self.prefix)?;

        // Find the highest semver tag reachable from rev_high
        let tag_high = repo
            .find_last_version(&rev_high, self.ignore_prereleases, &semvers)
            .with_context(|| {
                format!("Could not find the last version for revision {}", &self.rev)
            })?;

        // Find the highest semver tag reachable from rev_low (if set)
        let tag_low = match &rev_low {
            Some(rev_low) => repo.find_last_version(rev_low, self.ignore_prereleases, &semvers)?,
            None => None,
        };

        match tag_high {
            Some(tag_high) => {
                // Save the full semvers list for finding the lower boundary tag
                // when --max-* flags limit the visible tags.
                let all_semvers = semvers.clone();

                // Filter semver tags to those within the range:
                // - version > tag_low.version (strictly above the lower boundary tag)
                // - version <= tag_high.version (up to and including the upper boundary tag)
                let mut sem_ver_iter: Box<dyn Iterator<Item = (semver::Version, _)>> =
                    Box::new(semvers.into_iter().filter(|(version, _)| {
                        version <= &tag_high.0
                            && tag_low
                                .as_ref()
                                .map_or(true, |(low_ver, _)| version > low_ver)
                    }));
                if self.max_majors != u64::MAX {
                    sem_ver_iter = Box::new(sem_ver_iter.max_majors_iter(self.max_majors));
                }
                if self.max_minors != u64::MAX {
                    sem_ver_iter = Box::new(sem_ver_iter.max_minors_iter(self.max_minors));
                }
                if self.max_patches != u64::MAX {
                    sem_ver_iter = Box::new(sem_ver_iter.max_patches_iter(self.max_patches));
                }
                let semver_data: Vec<(_, _)> = sem_ver_iter.collect::<Vec<_>>();

                let semvers: Vec<Rev<_>> = semver_data
                    .into_iter()
                    .map(|(v, id)| Rev(Some(id), Some(v)))
                    .collect::<Vec<_>>();

                let mut revs = Vec::with_capacity(semvers.len() + 2);

                // If rev_high commit is not the same as tag_high commit,
                // add an unreleased section for commits between tag_high and rev_high
                if rev_high.id() != tag_high.1.id() {
                    revs.push(Rev(Some(rev_high), None));
                }

                revs.extend(semvers);

                // Add the lower boundary if there are sections above it.
                // The lower boundary is needed for windows(2) to produce pairs.
                if !revs.is_empty() {
                    if let Some(ref rev_low_commit) = rev_low {
                        if let Some((low_ver, low_commit)) = &tag_low {
                            if low_commit.id() == rev_low_commit.id() {
                                // rev_low is exactly a tag: add it as a versioned boundary
                                revs.push(Rev(Some(low_commit.clone()), Some(low_ver.clone())));
                            } else {
                                // rev_low is between tags: add it as an unversioned boundary
                                revs.push(Rev(Some(rev_low_commit.clone()), None));
                            }
                        } else {
                            // No tag_low found (rev_low is before any tags): add as unversioned
                            revs.push(Rev(Some(rev_low_commit.clone()), None));
                        }
                    } else {
                        // No rev_low specified: find the next tag below the last filtered one
                        // to use as the lower boundary. If there are no more tags, extend to root.
                        let last_filtered_ver = revs.last().and_then(|r| r.1.as_ref());
                        let next_tag_below = last_filtered_ver.and_then(|last_ver| {
                            all_semvers.iter().find(|(v, _)| v < last_ver).cloned()
                        });
                        if let Some((below_ver, below_commit)) = next_tag_below {
                            revs.push(Rev(Some(below_commit), Some(below_ver)));
                        } else {
                            // No more tags below: the last section extends to root
                            revs.push(Rev(None, None));
                        }
                    }
                }

                for w in revs.windows(2).map(|w| (w[0].clone(), w[1].clone())) {
                    let context = transformer.transform(w.0, w.1)?;
                    if !self.skip_empty || !context.context.commit_groups.is_empty() {
                        writer.write_template(&context)?;
                    }
                }
            }
            None => {
                // No tags found reachable from rev_high: show a single unreleased section
                let context = transformer.transform(Rev(Some(rev_high), None), Rev(None, None))?;
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
