use std::{borrow::Cow, collections::HashMap, convert::Infallible};

use bstr::ByteSlice;
use gix::{
    hashtable::Equivalent, object::tree::diff::Action, traverse::commit::ParentIds,
    worktree::stack::state::attributes::Source, Pathspec,
};
use jiff::{
    tz::{Offset, TimeZone},
    Timestamp,
};

use super::{Commit, CommitTrait, Repo, RevWalkOptions};
use crate::error::ConvcoError;

impl CommitTrait for gix::Commit<'_> {
    type ObjectId = gix::ObjectId;

    fn short_id(&self) -> String {
        self.short_id().unwrap().to_string()
    }

    fn commit_message(&self) -> Result<Cow<str>, ConvcoError> {
        let commit = self;
        let commit = commit.message().unwrap().title.to_str_lossy();
        Ok(Cow::Owned(commit.into_owned()))
    }

    fn id(&self) -> String {
        self.id.to_string()
    }

    fn oid(&self) -> Self::ObjectId {
        self.id
    }

    fn commit_time(&self) -> Result<jiff::Zoned, ConvcoError> {
        let time = self.time()?;
        let unix_time = time.seconds;
        let offset = time.offset;
        let timestamp = Timestamp::from_second(unix_time)?;
        let tz = TimeZone::fixed(Offset::from_seconds(offset)?);

        Ok(timestamp.to_zoned(tz))
    }
}

impl<'repo> Repo<'repo> for gix::Repository {
    type CommitTrait = gix::Commit<'repo>;

    fn open() -> Result<Self, ConvcoError> {
        Ok(gix::open(".")?)
    }

    fn url(&self, remote: &str) -> Result<Option<String>, ConvcoError> {
        Ok(self
            .find_remote(remote)?
            .url(gix::remote::Direction::Fetch)
            .map(ToString::to_string))
    }

    fn find_last_version(
        &'repo self,
        commit: &Self::CommitTrait,
        ignore_prereleases: bool,
        semvers: &[(semver::Version, Self::CommitTrait)],
    ) -> Result<Option<(semver::Version, Self::CommitTrait)>, ConvcoError> {
        let tips = [commit.id];
        let platform = self
            .rev_walk(tips)
            .sorting(gix::revision::walk::Sorting::BreadthFirst);
        let semvers = semvers
            .iter()
            .filter(|(version, _)| {
                if ignore_prereleases {
                    version.pre.is_empty()
                } else {
                    true
                }
            })
            .map(move |(version, oid)| (oid.id, (version, oid)))
            .collect::<HashMap<_, _>>();
        Ok(platform
            .all()?
            .flatten()
            .find_map(|info| semvers.get(&info.id)))
        .map(|vc| vc.map(|(v, c)| ((*v).clone(), (*c).clone())))
    }

    fn revwalk(
        &'repo self,
        options: RevWalkOptions<'repo, Self::CommitTrait>,
    ) -> Result<
        Box<
            dyn Iterator<Item = Result<Commit<Self::CommitTrait>, (ConvcoError, Self::CommitTrait)>>
                + 'repo,
        >,
        ConvcoError,
    > {
        let commit = options.to_rev.id;
        let tips = [commit];
        let boundary: Vec<_> = options.from_rev.iter().map(|from| from.id).collect();
        let mut platform = self
            .rev_walk(tips)
            .sorting(gix::revision::walk::Sorting::BreadthFirst);
        if options.first_parent {
            platform = platform.first_parent_only();
        }
        let check_changes = !options.paths.is_empty();
        let mut pathspec = self
            .pathspec(
                true,
                options.paths.into_iter().map(bstr::BString::from),
                true,
                &self.index().unwrap(),
                Source::IdMapping,
            )
            .unwrap();
        let mut revwalk: Box<dyn Iterator<Item = _>> = Box::new(
            platform
                .selected(move |oid| !boundary.iter().any(|rev| *rev == oid))?
                .flatten()
                .flat_map(move |info| {
                    let commit = info.object().ok()?;
                    Some((info, commit))
                }),
        );
        if options.no_merge_commits {
            revwalk = Box::new(revwalk.filter(move |(info, _)| info.parent_ids.len() <= 1));
        }
        if check_changes {
            revwalk = Box::new(revwalk.filter(move |(info, commit)| {
                self.commit_changes_path(commit, &info.parent_ids, &mut pathspec)
            }));
        }
        let revwalk: Box<dyn Iterator<Item = _>> = if options.no_revert_commits {
            Box::new(revwalk.filter_map(move |(_, commit)| {
                let msg = commit.message_raw().ok()?.to_str().ok()?;

                if msg.starts_with("Revert \"") {
                    return None;
                }

                Some(match options.parser.parse(msg) {
                    Ok(conventional_commit) => Ok(Commit {
                        conventional_commit,
                        commit,
                    }),
                    Err(e) => Err((e.into(), commit)),
                })
            }))
        } else {
            Box::new(revwalk.filter_map(move |(_, commit)| {
                let msg = commit.message_raw().ok()?.to_str().ok()?;

                Some(match options.parser.parse(msg) {
                    Ok(conventional_commit) => Ok(Commit {
                        conventional_commit,
                        commit,
                    }),
                    Err(e) => Err((e.into(), commit)),
                })
            }))
        };

        Ok(revwalk)
    }

    fn semver_tags(
        &'repo self,
        prefix: &str,
    ) -> Result<Vec<(semver::Version, Self::CommitTrait)>, ConvcoError> {
        let mut versions = self
            .references()?
            .tags()?
            .flatten()
            .filter_map(|mut tag| {
                let name = tag.name().as_bstr().strip_prefix(b"refs/tags/").unwrap();
                if name.starts_with(prefix.as_bytes()) {
                    let name = name.strip_prefix(prefix.as_bytes()).unwrap();
                    match semver::Version::parse(name.to_str().unwrap()) {
                        Ok(version) => Some((
                            version,
                            tag.peel_to_commit()
                                .unwrap()
                                .detach()
                                .attach(self)
                                .into_commit(),
                        )),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        versions.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(versions)
    }

    fn revparse_single(&'repo self, spec: &str) -> Result<Self::CommitTrait, ConvcoError> {
        Ok(self.rev_parse_single(spec)?.object()?.peel_to_commit()?)
    }
}

trait GixExt {
    fn commit_changes_path(
        &self,
        commit: &gix::Commit,
        parent_ids: &ParentIds,
        pathspec: &mut Pathspec,
    ) -> bool;
}

impl GixExt for gix::Repository {
    fn commit_changes_path(
        &self,
        commit: &gix::Commit,
        parent_ids: &ParentIds,
        pathspec: &mut Pathspec,
    ) -> bool {
        let Ok(new_tree) = commit.tree() else {
            return false;
        };

        let mut contains_changes = false;
        if parent_ids.is_empty() {
            let empty_tree = self.empty_tree();
            let Ok(mut changes) = empty_tree.changes() else {
                return false;
            };
            let _ = changes.for_each_to_obtain_tree(&new_tree, |change| {
                let is_file_change = change.entry_mode().is_blob();
                if is_file_change && pathspec.is_included(change.location(), None) {
                    contains_changes = true;
                    Ok::<Action, Infallible>(Action::Continue)
                } else {
                    Ok(Action::Continue)
                }
            });
            contains_changes
        } else {
            for parent_id in commit.parent_ids() {
                let other_tree = self.find_commit(parent_id).unwrap().tree().unwrap();
                let _ =
                    other_tree
                        .changes()
                        .unwrap()
                        .for_each_to_obtain_tree(&new_tree, |change| {
                            let is_file_change = change.entry_mode().is_blob();
                            if is_file_change && pathspec.is_included(change.location(), None) {
                                contains_changes = true;
                                Ok::<Action, Infallible>(Action::Cancel)
                            } else {
                                Ok(Action::Continue)
                            }
                        });
                if contains_changes {
                    return true;
                }
            }
            false
        }
    }
}
