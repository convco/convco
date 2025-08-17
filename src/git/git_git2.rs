use std::{borrow::Cow, collections::HashSet};

use bstr::ByteSlice;
use git2::DiffOptions;
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
            revwalk =
                Box::new(revwalk.filter(move |commit| {
                    self.commit_changes_path(commit, options.paths.as_slice())
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
    fn commit_changes_path(&self, commit: &git2::Commit, paths: &[String]) -> bool;
}

impl Git2Ext for git2::Repository {
    fn commit_changes_path(&self, commit: &git2::Commit, paths: &[String]) -> bool {
        let new_tree = commit.tree().ok();
        let new_tree = new_tree.as_ref();
        let mut opts = DiffOptions::new();

        paths.iter().for_each(|path| {
            opts.pathspec(path);
        });

        if commit.parent_count() == 0 {
            let old_tree = None;
            match self.diff_tree_to_tree(old_tree, new_tree, Some(&mut opts)) {
                Ok(diff) => diff.deltas().next().is_some(),
                Err(_) => false,
            }
        } else {
            let old_tree = commit.parent(0).and_then(|parent| parent.tree()).ok();
            let old_tree = old_tree.as_ref();
            match self.diff_tree_to_tree(old_tree, new_tree, Some(&mut opts)) {
                Ok(diff) => diff.deltas().next().is_some(),
                Err(_) => false,
            }
        }
    }
}
