use std::{borrow::Cow, fmt::Debug};

use crate::{
    conventional::commit::{CommitParser, ConventionalCommit, ParseError},
    error::ConvcoError,
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

pub trait Repo<'repo>: Sized {
    type CommitTrait: CommitTrait;
    fn open() -> Result<Self, ConvcoError>;

    fn find_last_version(
        &'repo self,
        commit: &Self::CommitTrait,
        ignore_prereleases: bool,
        semvers: &[(semver::Version, Self::CommitTrait)],
    ) -> Result<Option<(semver::Version, Self::CommitTrait)>, ConvcoError>;

    fn revwalk(
        &'repo self,
        options: RevWalkOptions<'repo, Self::CommitTrait>,
    ) -> Result<
        Box<
            dyn Iterator<Item = Result<Commit<Self::CommitTrait>, (ConvcoError, Self::CommitTrait)>>
                + 'repo,
        >,
        ConvcoError,
    >;

    /// Get the list of tags matching the prefix ordered by semver
    fn semver_tags(
        &'repo self,
        prefix: &str,
    ) -> Result<Vec<(semver::Version, Self::CommitTrait)>, ConvcoError>;

    fn revparse_single(&'repo self, spec: &str) -> Result<Self::CommitTrait, ConvcoError>;

    fn url(&self, remote: &str) -> Result<Option<String>, ConvcoError>;
}

macro_rules! define_max_component_iter {
    ($name:ident, $ext_trait:ident, $method:ident, $component:ident) => {
        pub struct $name<O: CommitTrait, I: Iterator<Item = (semver::Version, O)>> {
            inner: I,
            max_count: u64,
            current: u64,
        }

        pub trait $ext_trait<O: CommitTrait, I: Iterator<Item = (semver::Version, O)>> {
            fn $method(self, max_count: u64) -> $name<O, I>;
        }

        impl<O: CommitTrait, I: Iterator<Item = (semver::Version, O)>> Iterator for $name<O, I> {
            type Item = I::Item;

            fn next(&mut self) -> Option<Self::Item> {
                let next = self.inner.next()?;
                if self.current == u64::MAX {
                    self.current = next.0.$component;
                }
                if next.0.$component != self.current {
                    self.max_count -= 1;
                    if self.max_count == 0 {
                        return None;
                    }
                    self.current = next.0.$component;
                }
                Some(next)
            }
        }

        impl<I, O> $ext_trait<O, I> for I
        where
            I: Iterator<Item = (semver::Version, O)>,
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
define_max_component_iter!(MaxMajorsIter, MaxMajorsIterExt, max_majors_iter, major);
define_max_component_iter!(MaxMinorsIter, MaxMinorsIterExt, max_minors_iter, minor);
define_max_component_iter!(MaxPatchesIter, MaxPatchesIterExt, max_patches_iter, patch);

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

pub struct CommitIter<O, I: Iterator<Item = Result<Commit<O>, ParseError>>> {
    inner: I,
}

impl<O, I> Iterator for CommitIter<O, I>
where
    O: CommitTrait,
    I: Iterator<Item = Result<Commit<O>, ParseError>>,
{
    type Item = Result<Commit<O>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
