use std::{borrow::Cow, collections::HashSet};

use bstr::ByteSlice;
use git2::{Delta, Pathspec, PathspecFlags};
use jiff::{
    tz::{Offset, TimeZone},
    Timestamp,
};

use super::{Commit, CommitTrait, Repo, RevWalkIter, RevWalkOptions};
use crate::error::ConvcoError;

impl CommitTrait for git2::Commit<'_> {
    type ObjectId = git2::Oid;

    fn short_id(&self) -> String {
        self.as_object()
            .short_id()
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn commit_message(&self) -> Result<Cow<'_, str>, ConvcoError> {
        Ok(self.message_bytes().to_str_lossy())
    }

    fn id(&self) -> String {
        self.id().to_string()
    }

    fn oid(&self) -> Self::ObjectId {
        self.id()
    }

    fn commit_time(&self) -> Result<jiff::Zoned, ConvcoError> {
        let time = self.time();
        let unix_time = time.seconds();
        let offset = time.offset_minutes();
        let timestamp = Timestamp::from_second(unix_time)?;
        let tz = TimeZone::fixed(Offset::from_seconds(offset * 60)?);

        Ok(timestamp.to_zoned(tz))
    }
}

impl<'repo> Repo<'repo> for git2::Repository {
    type CommitTrait = git2::Commit<'repo>;

    fn open() -> Result<Self, ConvcoError> {
        Ok(git2::Repository::open_from_env()?)
    }

    fn url(&self, remote: &str) -> Result<Option<String>, ConvcoError> {
        match self.find_remote(remote) {
            Ok(remote) => Ok(Some(remote.url()?.to_owned())),
            Err(err) if err.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn find_last_version(
        &'repo self,
        commit: &Self::CommitTrait,
        ignore_prereleases: bool,
        semvers: &[(semver::Version, Self::CommitTrait)],
    ) -> Result<Option<(semver::Version, Self::CommitTrait)>, ConvcoError> {
        let mut revwalk = self.revwalk()?;
        revwalk.push(commit.id())?;
        let reachable = revwalk.flatten().collect::<HashSet<_>>();

        Ok(semvers
            .iter()
            .filter(|(version, _)| !ignore_prereleases || version.pre.is_empty())
            .find(|(_, commit)| reachable.contains(&commit.id()))
            .map(|(version, commit)| (version.clone(), commit.clone())))
    }

    fn revwalk(
        &'repo self,
        options: RevWalkOptions<'repo, Self::CommitTrait>,
    ) -> Result<RevWalkIter<'repo, Self::CommitTrait>, ConvcoError> {
        let commit = options.to_rev;
        let mut revwalk = self.revwalk()?;
        if options.first_parent {
            revwalk.simplify_first_parent()?;
        }
        for rev in options.from_rev {
            revwalk.hide(rev.id())?;
        }
        revwalk.push(commit.id())?;

        let mut revwalk: Box<dyn Iterator<Item = _>> =
            Box::new(revwalk.flatten().flat_map(|i| self.find_commit(i)));
        if options.no_merge_commits {
            revwalk = Box::new(revwalk.filter(move |commit| commit.parent_count() <= 1));
        }
        if !options.paths.is_empty() {
            let pathspec_filter = Git2PathspecFilter::new(options.paths.as_slice());
            revwalk = Box::new(revwalk.filter(move |commit| {
                pathspec_filter
                    .as_ref()
                    .is_some_and(|filter| self.commit_changes_path(commit, filter))
            }));
        }
        let revwalk: Box<dyn Iterator<Item = _>> = Box::new(revwalk.filter_map(move |commit| {
            let message = commit.message().ok().map(ToOwned::to_owned);
            message.and_then(|msg| {
                if options.no_revert_commits && msg.starts_with("Revert \"") {
                    return None;
                }
                Some(match options.parser.parse(&msg) {
                    Ok(conventional_commit) => Ok(Commit {
                        conventional_commit,
                        commit,
                    }),
                    Err(e) => Err((e.into(), commit)),
                })
            })
        }));

        Ok(revwalk)
    }

    fn semver_tags(
        &'repo self,
        prefix: &str,
    ) -> Result<Vec<(semver::Version, Self::CommitTrait)>, ConvcoError> {
        let mut versions = self
            .references_glob(&format!("refs/tags/{prefix}*"))?
            .flatten()
            .filter_map(|tag| {
                let name = tag.shorthand_bytes();
                let name = name.strip_prefix(prefix.as_bytes()).unwrap();
                name.to_str()
                    .ok()
                    .and_then(|name| semver::Version::parse(name).ok())
                    .and_then(|version| tag.peel_to_commit().ok().map(|commit| (version, commit)))
            })
            .collect::<Vec<_>>();
        versions.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(versions)
    }

    fn revparse_single(&'repo self, spec: &str) -> Result<Self::CommitTrait, ConvcoError> {
        Ok(self.revparse_single(spec)?.peel_to_commit()?)
    }

    fn revision_time(
        &'repo self,
        spec: &str,
        commit: &Self::CommitTrait,
    ) -> Result<jiff::Zoned, ConvcoError> {
        let object = self.revparse_single(spec)?;
        if let Some(time) = object.as_tag().and_then(|tag| tag.tagger()).map(|tagger| {
            let time = tagger.when();
            (time.seconds(), time.offset_minutes() * 60)
        }) {
            zoned_from_git_time(time.0, time.1)
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

trait Git2Ext {
    fn commit_changes_path(&self, commit: &git2::Commit, filter: &Git2PathspecFilter) -> bool;
}

impl Git2Ext for git2::Repository {
    fn commit_changes_path(&self, commit: &git2::Commit, filter: &Git2PathspecFilter) -> bool {
        let new_tree = commit.tree().ok();
        let new_tree = new_tree.as_ref();

        let diff = if commit.parent_count() == 0 {
            let old_tree = None;
            self.diff_tree_to_tree(old_tree, new_tree, None)
        } else {
            let old_tree = commit.parent(0).and_then(|parent| parent.tree()).ok();
            let old_tree = old_tree.as_ref();
            self.diff_tree_to_tree(old_tree, new_tree, None)
        };

        let Ok(diff) = diff else {
            return false;
        };

        filter.matches(&diff)
    }
}

struct Git2PathspecFilter {
    include: Option<Pathspec>,
    exclude: Option<Pathspec>,
}

impl Git2PathspecFilter {
    fn new(paths: &[String]) -> Option<Self> {
        let include_paths = paths
            .iter()
            .filter(|path| !is_exclude_pathspec(path))
            .collect::<Vec<_>>();
        let exclude_paths = paths
            .iter()
            .filter_map(|path| positive_pathspec_from_exclude(path))
            .collect::<Vec<_>>();

        let include = if include_paths.is_empty() {
            None
        } else {
            Some(Pathspec::new(include_paths).ok()?)
        };
        let exclude = if exclude_paths.is_empty() {
            None
        } else {
            Some(Pathspec::new(&exclude_paths).ok()?)
        };

        (include.is_some() || exclude.is_some()).then_some(Self { include, exclude })
    }

    fn matches(&self, diff: &git2::Diff<'_>) -> bool {
        diff.deltas().any(|delta| {
            let old_path = (delta.status() != Delta::Added)
                .then(|| delta.old_file().path())
                .flatten();
            let new_path = (delta.status() != Delta::Deleted)
                .then(|| delta.new_file().path())
                .flatten();

            old_path.into_iter().chain(new_path).any(|path| {
                let included = self
                    .include
                    .as_ref()
                    .is_none_or(|pathspec| pathspec.matches_path(path, PathspecFlags::DEFAULT));
                let excluded = self
                    .exclude
                    .as_ref()
                    .is_some_and(|pathspec| pathspec.matches_path(path, PathspecFlags::DEFAULT));

                included && !excluded
            })
        })
    }
}

fn is_exclude_pathspec(pathspec: &str) -> bool {
    pathspec.starts_with(":!")
        || pathspec.starts_with(":^")
        || long_magic_contains_exclude(pathspec)
}

fn long_magic_contains_exclude(pathspec: &str) -> bool {
    let Some(magic) = pathspec
        .strip_prefix(":(")
        .and_then(|pathspec| pathspec.split_once(')').map(|(magic, _)| magic))
    else {
        return false;
    };

    magic.split(',').any(|token| token == "exclude")
}

fn positive_pathspec_from_exclude(pathspec: &str) -> Option<String> {
    if let Some(pathspec) = pathspec
        .strip_prefix(":!")
        .or_else(|| pathspec.strip_prefix(":^"))
    {
        return Some(pathspec.to_owned());
    }

    let pathspec = pathspec.strip_prefix(":(")?;
    let (magic, pattern) = pathspec.split_once(')')?;
    let magic = magic
        .split(',')
        .filter(|token| *token != "exclude")
        .collect::<Vec<_>>();

    Some(if magic.is_empty() {
        pattern.to_owned()
    } else {
        format!(":({}){}", magic.join(","), pattern)
    })
}
