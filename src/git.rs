use std::{borrow::Cow, fmt::Debug};

use crate::{
    conventional::commit::{CommitParser, ConventionalCommit},
    error::ConvcoError,
    VersionScheme, VersionTag,
};

#[cfg(feature = "git2")]
mod git_git2;
#[cfg(feature = "gix")]
mod git_gix;

#[cfg(feature = "git2")]
pub fn open_repo() -> Result<git2::Repository, ConvcoError> {
    Repo::open()
}

#[cfg(all(not(feature = "git2"), feature = "gix"))]
pub fn open_repo() -> Result<gix::Repository, ConvcoError> {
    Repo::open()
}

#[derive(Debug)]
pub struct Commit<C> {
    pub conventional_commit: ConventionalCommit,
    pub commit: C,
}

pub trait CommitTrait: Debug + Clone {
    type ObjectId;
    fn short_id(&self) -> String;
    fn id(&self) -> String;
    fn oid(&self) -> Self::ObjectId;
    fn commit_message(&self) -> Result<Cow<'_, str>, ConvcoError>;
    fn commit_time(&self) -> Result<jiff::Zoned, ConvcoError>;
}

pub type RevWalkIter<'repo, C> =
    Box<dyn Iterator<Item = Result<Commit<C>, (ConvcoError, C)>> + 'repo>;

pub trait Repo<'repo>: Sized {
    type CommitTrait: CommitTrait;
    fn open() -> Result<Self, ConvcoError>;

    fn find_last_version(
        &'repo self,
        commit: &Self::CommitTrait,
        ignore_prereleases: bool,
        versions: &[(VersionTag, Self::CommitTrait)],
    ) -> Result<Option<(VersionTag, Self::CommitTrait)>, ConvcoError>;

    fn revwalk(
        &'repo self,
        options: RevWalkOptions<'repo, Self::CommitTrait>,
    ) -> Result<RevWalkIter<'repo, Self::CommitTrait>, ConvcoError>;

    /// Get the list of tags matching the prefix ordered by semver
    fn semver_tags(
        &'repo self,
        prefix: &str,
    ) -> Result<Vec<(semver::Version, Self::CommitTrait)>, ConvcoError>;

    /// Get the list of tags matching the prefix ordered by the selected scheme
    fn version_tags(
        &'repo self,
        prefix: &str,
        scheme: &VersionScheme,
    ) -> Result<Vec<(VersionTag, Self::CommitTrait)>, ConvcoError>;

    fn revparse_single(&'repo self, spec: &str) -> Result<Self::CommitTrait, ConvcoError>;

    fn revision_time(
        &'repo self,
        spec: &str,
        commit: &Self::CommitTrait,
    ) -> Result<jiff::Zoned, ConvcoError>;

    fn url(&self, remote: &str) -> Result<Option<String>, ConvcoError>;
}

macro_rules! define_max_component_iter {
    ($name:ident, $ext_trait:ident, $method:ident, $component:literal) => {
        pub struct $name<O: CommitTrait, I: Iterator<Item = (VersionTag, O)>> {
            inner: I,
            max_count: u64,
            current: u64,
        }

        pub trait $ext_trait<O: CommitTrait, I: Iterator<Item = (VersionTag, O)>> {
            fn $method(self, max_count: u64) -> $name<O, I>;
        }

        impl<O: CommitTrait, I: Iterator<Item = (VersionTag, O)>> Iterator for $name<O, I> {
            type Item = I::Item;

            fn next(&mut self) -> Option<Self::Item> {
                let next = self.inner.next()?;
                if self.current == u64::MAX {
                    self.current = next.0.component($component);
                }
                if next.0.component($component) != self.current {
                    self.max_count -= 1;
                    if self.max_count == 0 {
                        return None;
                    }
                    self.current = next.0.component($component);
                }
                Some(next)
            }
        }

        impl<I, O> $ext_trait<O, I> for I
        where
            I: Iterator<Item = (VersionTag, O)>,
            O: CommitTrait,
        {
            fn $method(self, max_count: u64) -> $name<O, I> {
                $name {
                    inner: self,
                    max_count,
                    current: u64::MAX,
                }
            }
        }
    };
}
define_max_component_iter!(MaxMajorsIter, MaxMajorsIterExt, max_majors_iter, 0);
define_max_component_iter!(MaxMinorsIter, MaxMinorsIterExt, max_minors_iter, 1);
define_max_component_iter!(MaxPatchesIter, MaxPatchesIterExt, max_patches_iter, 2);

#[derive(Clone, Debug)]
pub struct RevWalkOptions<'a, C> {
    /// the ancestor commits tho hide
    pub from_rev: Vec<C>,
    /// the commit to start the walk from. Defaults to HEAD.
    pub to_rev: C,
    /// Only follow first parent
    pub first_parent: bool,
    /// Include or exclude merge commits (more than 1 parent)
    pub no_merge_commits: bool,
    /// Ignore revert commits (Starting with Revert..)
    pub no_revert_commits: bool,
    /// Paths to include, usefull for monorepos
    pub paths: Vec<String>,
    pub parser: &'a CommitParser,
}
