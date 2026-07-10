use std::{borrow::Cow, collections::HashSet, convert::Infallible};

use bstr::ByteSlice;
use gix::{
    object::tree::diff::Action, traverse::commit::ParentIds,
    worktree::stack::state::attributes::Source, Pathspec,
};
use jiff::{
    tz::{Offset, TimeZone},
    Timestamp,
};

use super::{Commit, CommitTrait, Repo, RevWalkIter, RevWalkOptions};
use crate::{error::ConvcoError, VersionScheme, VersionTag};

impl CommitTrait for gix::Commit<'_> {
    type ObjectId = gix::ObjectId;

    fn short_id(&self) -> String {
        self.short_id().unwrap().to_string()
    }

    fn commit_message(&self) -> Result<Cow<'_, str>, ConvcoError> {
        Ok(self.message_raw()?.to_str_lossy())
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
        Ok(gix::discover(".")?)
    }

    fn url(&self, remote: &str) -> Result<Option<String>, ConvcoError> {
        match self.find_remote(remote) {
            Ok(remote) => Ok(remote
                .url(gix::remote::Direction::Fetch)
                .map(ToString::to_string)),
            Err(gix::remote::find::existing::Error::NotFound { .. }) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn find_last_version(
        &'repo self,
        commit: &Self::CommitTrait,
        ignore_prereleases: bool,
        versions: &[(VersionTag, Self::CommitTrait)],
    ) -> Result<Option<(VersionTag, Self::CommitTrait)>, ConvcoError> {
        let tips = [commit.id];
        let platform = self
            .rev_walk(tips)
            .sorting(gix::revision::walk::Sorting::BreadthFirst);
        let reachable = platform
            .all()?
            .flatten()
            .map(|info| info.id)
            .collect::<HashSet<_>>();

        Ok(versions
            .iter()
            .filter(|(version, _)| !ignore_prereleases || !version.is_prerelease())
            .find(|(_, commit)| reachable.contains(&commit.id))
            .map(|(version, commit)| (version.clone(), commit.clone())))
    }

    fn revwalk(
        &'repo self,
        options: RevWalkOptions<'repo, Self::CommitTrait>,
    ) -> Result<RevWalkIter<'repo, Self::CommitTrait>, ConvcoError> {
        let commit = options.to_rev.id;
        let tips = [commit];
        let boundary: Vec<_> = options.from_rev.iter().map(|from| from.id).collect();
        let mut platform = self
            .rev_walk(tips)
            .sorting(gix::revision::walk::Sorting::BreadthFirst);
        if options.first_parent {
            platform = platform.first_parent_only();
        }
        let paths = options.paths;
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
        if !paths.is_empty() {
            let mut pathspec = self
                .pathspec(
                    true,
                    paths.into_iter().map(bstr::BString::from),
                    true,
                    &self.index().unwrap(),
                    Source::IdMapping,
                )
                .unwrap();
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
        let versions = self.version_tags(prefix, &VersionScheme::Semver)?;
        Ok(versions
            .into_iter()
            .filter_map(|(version, commit)| match version {
                VersionTag::Semver(version) => Some((version, commit)),
                VersionTag::Calver(_) => None,
            })
            .collect())
    }

    fn version_tags(
        &'repo self,
        prefix: &str,
        scheme: &VersionScheme,
    ) -> Result<Vec<(VersionTag, Self::CommitTrait)>, ConvcoError> {
        let mut versions = self
            .references()?
            .tags()?
            .flatten()
            .filter_map(|mut tag| {
                let name = tag.name().as_bstr().strip_prefix(b"refs/tags/").unwrap();
                if name.starts_with(prefix.as_bytes()) {
                    let name = name.strip_prefix(prefix.as_bytes()).unwrap();
                    let name = name.to_str().ok()?;
                    let version = match scheme {
                        VersionScheme::Semver => {
                            VersionTag::Semver(semver::Version::parse(name).ok()?)
                        }
                        VersionScheme::Calver(format) => {
                            VersionTag::Calver(format.parse_version(name)?)
                        }
                    };
                    let commit = tag
                        .peel_to_commit()
                        .ok()?
                        .detach()
                        .attach(self)
                        .into_commit();
                    Some((version, commit))
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

    fn revision_time(
        &'repo self,
        spec: &str,
        commit: &Self::CommitTrait,
    ) -> Result<jiff::Zoned, ConvcoError> {
        if let Some(time) = self
            .rev_parse_single(spec)
            .ok()
            .and_then(|revision| revision.object().ok())
            .and_then(|object| object.try_into_tag().ok())
            .and_then(|tag| {
                tag.tagger()
                    .ok()
                    .flatten()
                    .and_then(|tagger| tagger.time().ok())
            })
        {
            zoned_from_git_time(time.seconds, time.offset)
        } else {
            commit.commit_time()
        }
    }
}

fn zoned_from_git_time(seconds: i64, offset_seconds: i32) -> Result<jiff::Zoned, ConvcoError> {
    let timestamp = Timestamp::from_second(seconds)?;
    let tz = TimeZone::fixed(Offset::from_seconds(offset_seconds)?);

    Ok(timestamp.to_zoned(tz))
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
                    Ok::<Action, Infallible>(Action::Continue(()))
                } else {
                    Ok(Action::Continue(()))
                }
            });
            contains_changes
        } else {
            let Some(parent_id) = commit.parent_ids().next() else {
                return false;
            };
            let Ok(parent_commit) = self.find_commit(parent_id) else {
                return false;
            };
            let Ok(other_tree) = parent_commit.tree() else {
                return false;
            };
            let Ok(mut changes) = other_tree.changes() else {
                return false;
            };
            let _ = changes.for_each_to_obtain_tree(&new_tree, |change| {
                let is_file_change = change.entry_mode().is_blob();
                if is_file_change && pathspec.is_included(change.location(), None) {
                    contains_changes = true;
                    Ok::<Action, Infallible>(Action::Break(()))
                } else {
                    Ok(Action::Continue(()))
                }
            });
            contains_changes
        }
    }
}
