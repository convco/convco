use std::{cmp::Ordering, collections::HashMap, path::PathBuf};

use git2::{Commit, Diff, Error, Object, Oid, Repository, Revwalk};
use semver::Version;

use crate::semver::SemVer;

/// git helper for common operations
pub(crate) struct GitHelper {
    pub(crate) repo: Repository,
    version_map: HashMap<Oid, Vec<VersionAndTag>>,
}

#[derive(Clone, Debug)]
pub(crate) struct VersionAndTag {
    pub(crate) tag: String,
    pub(crate) version: SemVer,
    pub(crate) commit_sha: String,
}

impl Eq for VersionAndTag {}

impl PartialEq for VersionAndTag {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}

impl PartialOrd for VersionAndTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionAndTag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl GitHelper {
    pub(crate) fn new(prefix: &str) -> Result<Self, Error> {
        let repo = Repository::open_from_env()?;
        let version_map = make_oid_version_map(&repo, prefix);

        Ok(Self { repo, version_map })
    }

    /// Get the last version (can be pre-release) for the given revision.
    ///
    /// Arguments:
    ///
    /// - rev: A single commit rev spec
    /// - ignore_prereleases: If true, ignore pre-release versions
    pub(crate) fn find_last_version(
        &self,
        rev: &str,
        ignore_prereleases: bool,
    ) -> Result<Option<VersionAndTag>, Error> {
        let rev = self.repo.revparse_single(rev)?.peel_to_commit()?;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(rev.id())?;
        let mut version: Vec<&VersionAndTag> = revwalk
            .flatten()
            .filter_map(|oid| {
                self.version_map.get(&oid).map(|v| {
                    v.iter()
                        .filter(|v| !ignore_prereleases || !v.version.is_prerelease())
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .collect();
        version.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(version.first().cloned().cloned())
    }

    /// Returns a sorted vector with the lowest version at index `0`.
    pub(crate) fn versions_from(&self, version: &VersionAndTag) -> Vec<&VersionAndTag> {
        let mut values: Vec<&VersionAndTag> = self.version_map.values().flatten().collect();
        values.retain(|v| *v < version && !v.version.is_prerelease());
        values.sort();
        values
    }

    pub(crate) fn revwalk(&self) -> Result<Revwalk<'_>, Error> {
        self.repo.revwalk()
    }

    pub(crate) fn find_commit(&self, oid: Oid) -> Result<Commit<'_>, Error> {
        self.repo.find_commit(oid)
    }

    pub(crate) fn ref_to_commit(&self, r#ref: &str) -> Result<Commit<'_>, Error> {
        self.repo.revparse_single(r#ref)?.peel_to_commit()
    }

    pub(crate) fn same_commit(&self, ref_a: &str, ref_b: &str) -> bool {
        ref_a == ref_b
            || match (self.ref_to_commit(ref_a), self.ref_to_commit(ref_b)) {
                (Ok(a), Ok(b)) => a.id() == b.id(),
                _ => false,
            }
    }

    /// return the host of the repo
    pub(crate) fn url(&self) -> Result<Option<String>, Error> {
        Ok(self
            .repo
            .find_remote("origin")?
            .url()
            .map(|s| s.to_string()))
    }

    pub(crate) fn commit_updates_any_path(&self, commit: &Commit, paths: &[PathBuf]) -> bool {
        if paths.is_empty() {
            return true;
        }

        let tree = commit.tree().ok();
        let parent_tree = commit.parent(0).and_then(|item| item.tree()).ok();
        self.repo
            .diff_tree_to_tree(parent_tree.as_ref(), tree.as_ref(), None)
            .map(|diff| diff_updates_any_path(&diff, paths))
            .unwrap_or(false)
    }
}

pub(crate) fn filter_merge_commits(commit: &git2::Commit, merges: bool) -> bool {
    merges || commit.parent_count() <= 1
}

pub(crate) fn filter_revert_commits(commit: &git2::Commit, ignore_reverts: bool) -> bool {
    if ignore_reverts {
        return commit
            .message()
            .map(|m| !m.starts_with("Revert \""))
            .unwrap_or(true);
    }
    true
}

/// Build a hashmap that contains Commit `Oid` as key and a vector of `Version` as value.
/// Can be used to easily walk a graph and check if it is a version.
fn make_oid_version_map(repo: &Repository, prefix: &str) -> HashMap<Oid, Vec<VersionAndTag>> {
    let tags = repo
        .tag_names(Some(format!("{}*.*.*", prefix).as_str()))
        .expect("some array");
    let mut map = HashMap::<_, Vec<_>>::new();
    for tag in tags.iter().flatten().filter(|tag| tag.starts_with(prefix)) {
        if let Ok(oid) = repo.revparse_single(tag).map(object_to_target_commit_id) {
            if let Ok(version) = Version::parse(tag.trim_start_matches(prefix)) {
                map.entry(oid).or_default().push(VersionAndTag {
                    tag: tag.to_owned(),
                    version: SemVer(version),
                    commit_sha: oid.to_string(),
                });
            }
        }
    }
    map
}

fn object_to_target_commit_id(obj: Object<'_>) -> Oid {
    if let Some(tag) = obj.as_tag() {
        tag.target_id()
    } else {
        obj.id()
    }
}

fn diff_updates_any_path(diff: &Diff, paths: &[PathBuf]) -> bool {
    let mut update_any_path = false;

    diff.foreach(
        &mut |delta, _progress| {
            if let Some(file) = delta.new_file().path() {
                update_any_path |= paths.iter().any(|path| file.starts_with(path));
            }

            !update_any_path
        },
        None,
        None,
        None,
    )
    .ok();

    update_any_path
}
