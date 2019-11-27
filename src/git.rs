use git2::{Commit, Error, Object, Oid, Repository, Revwalk};
use semver::Version;
use std::{cmp::Ordering, collections::HashMap};

/// git helper for common operations
pub(crate) struct GitHelper {
    repo: Repository,
    version_map: HashMap<Oid, VersionAndTag>,
}

#[derive(Clone, Debug)]
pub(crate) struct VersionAndTag {
    pub(crate) tag: String,
    pub(crate) version: Version,
}

impl Eq for VersionAndTag {}

impl PartialEq for VersionAndTag {
    fn eq(&self, other: &Self) -> bool {
        self.version.eq(&other.version)
    }
}

impl PartialOrd for VersionAndTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.version.partial_cmp(&other.version)
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
    /// - prefix: The version prefix
    pub(crate) fn find_last_version(&self, rev: &str) -> Result<Option<VersionAndTag>, Error> {
        let rev = self.repo.revparse_single(rev)?.peel_to_commit()?;
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(rev.id())?;
        let mut version: Vec<&VersionAndTag> = revwalk
            .flatten()
            .filter_map(|oid| self.version_map.get(&oid))
            .collect();
        version.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(version.first().cloned().cloned())
    }

    /// Returnes a sorted vector with the lowest version at index `0`.
    pub(crate) fn versions_from(&self, version: &VersionAndTag) -> Vec<&VersionAndTag> {
        let mut values: Vec<&VersionAndTag> = self.version_map.values().collect();
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
}

/// Build a hashmap that contains Commit `Oid` as key and `Version` as value.
/// Can be used to easily walk a graph and check if it is a version.
fn make_oid_version_map(repo: &Repository, prefix: &str) -> HashMap<Oid, VersionAndTag> {
    let tags = repo
        .tag_names(Some(format!("{}*.*.*", prefix).as_str()))
        .expect("some array");
    let mut map = HashMap::new();
    for tag in tags.iter().flatten().filter(|tag| tag.starts_with(prefix)) {
        if let Ok(oid) = repo
            .revparse_single(tag)
            .and_then(object_to_target_commit_id)
        {
            if let Ok(version) = Version::parse(tag.trim_start_matches(prefix)) {
                map.insert(
                    oid,
                    VersionAndTag {
                        tag: tag.to_owned(),
                        version,
                    },
                );
            }
        }
    }
    map
}

fn object_to_target_commit_id(obj: Object<'_>) -> Result<Oid, Error> {
    if let Some(tag) = obj.as_tag() {
        Ok(tag.target_id())
    } else {
        Ok(obj.id())
    }
}
