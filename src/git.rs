use std::{cmp::Ordering, collections::HashMap, path::PathBuf};

use git2::{Commit, Error, Object, Oid, Repository, Revwalk};
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

    /// Returns true if a commit should be considered relevant based on include/ignore path filters.
    ///
    /// - If `include_paths` is empty, any touched path matches the include condition.
    /// - If `ignore_paths` is non-empty, commits that touch files *only* under those paths are ignored.
    pub(crate) fn commit_updates_relevant_paths(
        &self,
        commit: &Commit,
        include_paths: &[PathBuf],
        ignore_paths: &[PathBuf],
    ) -> bool {
        if include_paths.is_empty() && ignore_paths.is_empty() {
            return true;
        }

        let tree = commit.tree().ok();
        let parent_tree = commit.parent(0).and_then(|item| item.tree()).ok();
        let diff = match self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), tree.as_ref(), None)
        {
            Ok(diff) => diff,
            Err(_) => return false,
        };

        let mut touched_any = false;
        let mut updates_included = include_paths.is_empty();
        let mut updates_non_ignored = ignore_paths.is_empty();

        diff.foreach(
            &mut |delta, _progress| {
                let mut consider_path = |p: &std::path::Path| {
                    touched_any = true;
                    if !updates_included {
                        updates_included |= include_paths.iter().any(|path| p.starts_with(path));
                    }
                    if !updates_non_ignored {
                        updates_non_ignored |= !ignore_paths.iter().any(|path| p.starts_with(path));
                    }
                };

                if let Some(p) = delta.new_file().path() {
                    consider_path(p);
                }
                if let Some(p) = delta.old_file().path() {
                    consider_path(p);
                }

                // Stop early if both conditions are already satisfied.
                !(updates_included && (updates_non_ignored || !touched_any))
            },
            None,
            None,
            None,
        )
        .ok();

        (include_paths.is_empty() || updates_included)
            && (ignore_paths.is_empty() || !touched_any || updates_non_ignored)
    }

    pub(crate) fn find_matching_prerelease(
        &self,
        last_version: &SemVer,
        prerelease: &semver::Prerelease,
        commit_sha: &str,
    ) -> Option<semver::Prerelease> {
        let mut prereleases = self
            .version_map
            .values()
            .flat_map(|vat| vat.iter())
            .filter(|vat| {
                vat.version.0.major == last_version.major()
                    && vat.version.0.minor == last_version.minor()
                    && vat.version.0.patch == last_version.patch()
                    && vat.commit_sha == commit_sha
                    && vat
                        .version
                        .0
                        .pre
                        .rsplit_once('.')
                        .filter(|pre| prerelease.as_str() == pre.0)
                        .is_some()
            })
            .map(|vat| vat.version.0.clone())
            .collect::<Vec<_>>();
        // sort by pre-release version
        prereleases.sort();
        // return the last pre-release version
        prereleases.last().cloned().map(|version| version.pre)
    }

    pub(crate) fn find_last_prerelease(
        &self,
        last_version: &SemVer,
        prerelease: &semver::Prerelease,
    ) -> Option<semver::Prerelease> {
        let mut prereleases = self
            .version_map
            .values()
            .flat_map(|vat| vat.iter())
            .map(|vat| &vat.version.0)
            .filter(|version| {
                version.major == last_version.major()
                    && version.minor == last_version.minor()
                    && version.patch == last_version.patch()
                    && version
                        .pre
                        .rsplit_once('.')
                        .filter(|pre| prerelease.as_str() == pre.0)
                        .is_some()
            })
            .collect::<Vec<_>>();
        // sort by pre-release version
        prereleases.sort();
        // return the last pre-release version
        prereleases.last().map(|version| version.pre.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_last_unordered_prerelease() {
        let git_helper = GitHelper {
            repo: Repository::open(".").unwrap(),
            version_map: HashMap::from([
                (
                    Oid::from_str("0001").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.1".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.1").unwrap()),
                        commit_sha: "0001".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0003").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.3".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.3").unwrap()),
                        commit_sha: "0003".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0002").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.2".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.2").unwrap()),
                        commit_sha: "0002".to_string(),
                    }],
                ),
            ]),
        };

        assert_eq!(
            git_helper.find_last_prerelease(
                &SemVer(Version::new(1, 2, 3)),
                &semver::Prerelease::new("rc").unwrap(),
            ),
            Some(semver::Prerelease::new("rc.3").unwrap())
        );
    }

    #[test]
    fn test_find_matching_prerelease() {
        let git_helper = GitHelper {
            repo: Repository::open(".").unwrap(),
            version_map: HashMap::from([
                (
                    Oid::from_str("0001").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.1".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.1").unwrap()),
                        commit_sha: "0001".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0003").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.3".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.3").unwrap()),
                        commit_sha: "0003".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0002").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.2".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.2").unwrap()),
                        commit_sha: "0002".to_string(),
                    }],
                ),
            ]),
        };

        assert_eq!(
            git_helper.find_matching_prerelease(
                &SemVer(Version::new(1, 2, 3)),
                &semver::Prerelease::new("rc").unwrap(),
                "0001",
            ),
            Some(semver::Prerelease::new("rc.1").unwrap())
        );
    }

    #[test]
    fn test_find_matching_prerelease_without_matching_release() {
        let git_helper = GitHelper {
            repo: Repository::open(".").unwrap(),
            version_map: HashMap::from([
                (
                    Oid::from_str("0001").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.1".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.1").unwrap()),
                        commit_sha: "0001".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0003").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-beta.1".to_string(),
                        version: SemVer(Version::parse("1.2.3-beta.1").unwrap()),
                        commit_sha: "0004".to_string(),
                    }],
                ),
                (
                    Oid::from_str("0002").unwrap(),
                    vec![VersionAndTag {
                        tag: "v1.2.3-rc.2".to_string(),
                        version: SemVer(Version::parse("1.2.3-rc.2").unwrap()),
                        commit_sha: "0002".to_string(),
                    }],
                ),
            ]),
        };

        assert_eq!(
            git_helper.find_matching_prerelease(
                &SemVer(Version::new(1, 2, 3)),
                &semver::Prerelease::new("rc").unwrap(),
                "0004",
            ),
            None
        );
    }
}
