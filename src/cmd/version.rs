use std::fmt;

use convco::{
    open_repo, CommitParser, CommitTrait, Config, ConvcoError, Increment, Repo, RevWalkOptions,
    Type,
};
use semver::Version;

use crate::{cli::VersionCommand, cmd::Command};

enum Label {
    /// Bump major version (0.1.0 -> 1.0.0)
    Major,
    /// Bump minor version (0.1.0 -> 0.2.0)
    Minor,
    /// Bump patch version (0.1.0 -> 0.1.1)
    Patch,
    /// Remove the pre-release extension; if any (0.1.0-dev.1 -> 0.1.0, 0.1.0 -> 0.1.0)
    Release,
    /// Output a pre-release version
    Prerelease,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Patch => write!(f, "patch"),
            Self::Release => write!(f, "release"),
            Self::Prerelease => write!(f, "prerelease"),
        }
    }
}

impl VersionCommand {
    fn get_version(
        &self,
        scope_regex: String,
        strip_regex: String,
        types: Vec<convco::Type>,
        mut initial_bump_version: Version,
    ) -> Result<(Version, Label, String), ConvcoError> {
        let repo = open_repo()?;
        let prefix = self.prefix.as_str();
        let ignore_prereleases = self.ignore_prereleases; // FIXME add?: || (self.bump && self.prerelease.is_empty());
        let semvers = repo.semver_tags(prefix)?;
        let rev = Repo::revparse_single(&repo, &self.rev)?;
        let last_version = repo.find_last_version(&rev, ignore_prereleases, &semvers)?;
        match last_version {
            None => {
                let commit_sha = Repo::revparse_single(&repo, &self.rev)?;
                let commit_sha = CommitTrait::short_id(&commit_sha);
                let mut version = Version::new(0, 0, 0);
                if self.bump {
                    if self.prerelease.is_empty() {
                        let label = match (
                            initial_bump_version.major,
                            initial_bump_version.minor,
                            initial_bump_version.patch,
                        ) {
                            (_, 0, 0) => Label::Major,
                            (_, _, 0) => Label::Minor,
                            _ => Label::Patch,
                        };
                        Ok((initial_bump_version, label, commit_sha))
                    } else {
                        initial_bump_version.increment_prerelease(&self.prerelease);
                        Ok((initial_bump_version, Label::Prerelease, commit_sha))
                    }
                } else if self.patch {
                    version.patch = 1;
                    Ok((version, Label::Patch, commit_sha))
                } else if self.minor {
                    version.minor = 1;
                    Ok((version, Label::Minor, commit_sha))
                } else if self.major {
                    version.major = 1;
                    Ok((version, Label::Major, commit_sha))
                } else {
                    Ok((version, Label::Patch, commit_sha))
                }
            }
            Some((mut version, commit)) => {
                let v = if self.major {
                    version.increment_major();
                    (version, Label::Major, CommitTrait::short_id(&commit))
                } else if self.minor {
                    version.increment_minor();
                    (version, Label::Minor, CommitTrait::short_id(&commit))
                } else if self.patch {
                    version.increment_patch();
                    (version, Label::Patch, CommitTrait::short_id(&commit))
                } else if self.bump {
                    if version.is_prerelease() {
                        if self.prerelease.is_empty() {
                            version.pre_clear();
                            version.build_clear();
                            (version, Label::Release, CommitTrait::short_id(&commit))
                        } else {
                            version.increment_prerelease(&self.prerelease);
                            (version, Label::Prerelease, CommitTrait::short_id(&commit))
                        }
                    } else {
                        let parser = CommitParser::builder()
                            .scope_regex(scope_regex)
                            .strip_regex(strip_regex)
                            .build();
                        self.find_bump_version(&repo, commit, version, &parser, &types)?
                    }
                } else {
                    (version, Label::Release, CommitTrait::short_id(&commit))
                };
                Ok(v)
            }
        }
    }

    fn find_bump_version<'a, R, C>(
        &self,
        repo: &'a R,
        commit: C,
        last_version: semver::Version,
        parser: &'a CommitParser,
        types: &[Type],
    ) -> Result<(Version, Label, String), ConvcoError>
    where
        R: Repo<'a, CommitTrait = C>,
        C: CommitTrait,
    {
        let mut last_version = last_version;
        let to_rev = repo.revparse_single(&self.rev)?;
        let options = RevWalkOptions {
            from_rev: vec![commit],
            to_rev,
            first_parent: false,
            no_merge_commits: false,
            no_revert_commits: false,
            paths: self.paths.clone(),
            parser,
        };
        let revwalk = repo.revwalk(options)?;
        let mut major = false;
        let mut minor = false;
        let mut patch = false;

        let major_version_zero = last_version.major == 0;
        let mut commit_sha = None;
        for commit in revwalk.flatten() {
            if commit_sha.is_none() {
                commit_sha = Some(commit.commit.short_id());
            }
            if commit.conventional_commit.is_breaking() {
                if major_version_zero {
                    minor = true;
                } else {
                    major = true;
                }
                break;
            }

            let option_commit_type = types
                .iter()
                .find(|x| x.r#type == commit.conventional_commit.r#type);

            if let Some(some_commit_type) = option_commit_type {
                match (&some_commit_type.increment, major_version_zero) {
                    (Increment::Major, _) => major = true,
                    (Increment::Minor, true) => patch = true,
                    (Increment::Minor, false) => minor = true,
                    (Increment::Patch, _) => patch = true,
                    _ => {}
                }
            }
        }
        let label = match (major, minor, patch) {
            (true, _, _) => {
                last_version.increment_major();
                Label::Major
            }
            (false, true, _) => {
                last_version.increment_minor();
                Label::Minor
            }
            (false, false, true) => {
                last_version.increment_patch();
                Label::Patch
            }
            // TODO what should be the behaviour? always increment patch? or stay on same version?
            _ => Label::Release,
        };
        let commit_sha = commit_sha.unwrap_or_default();
        if !self.prerelease.is_empty() {
            last_version.increment_prerelease(&self.prerelease);
        }
        Ok((last_version, label, commit_sha))
    }
}

impl Command for VersionCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let initial_bump_version = self
            .initial_bump_version
            .clone()
            .unwrap_or(config.initial_bump_version);
        let (version, label, commit_sha) = self.get_version(
            config.scope_regex,
            config.strip_regex,
            config.types,
            initial_bump_version,
        )?;
        if self.label {
            println!("{label}");
        } else if self.commit_sha {
            println!("{commit_sha}");
        } else if self.print_prefix {
            println!("{}{version}", self.prefix);
        } else {
            println!("{version}");
        }
        Ok(())
    }
}

trait VersionExt {
    fn increment_major(&mut self);
    fn increment_minor(&mut self);
    fn increment_patch(&mut self);
    fn increment_prerelease(&mut self, prerelease: &semver::Prerelease);
    fn pre_clear(&mut self);
    fn build_clear(&mut self);

    fn is_prerelease(&self) -> bool;
}
impl VersionExt for Version {
    fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_patch(&mut self) {
        self.patch += 1;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_prerelease(&mut self, prerelease: &semver::Prerelease) {
        if self.pre.is_empty() {
            self.pre = semver::Prerelease::new(format!("{prerelease}.1").as_str()).unwrap();
        } else {
            let next = self
                .pre
                .split_once('.')
                .and_then(|(_, number)| number.parse::<u64>().ok())
                .unwrap_or_default()
                + 1;
            self.pre = semver::Prerelease::new(format!("{prerelease}.{next}").as_str()).unwrap();
        }
    }

    fn build_clear(&mut self) {
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn pre_clear(&mut self) {
        self.pre = semver::Prerelease::EMPTY
    }

    fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}
