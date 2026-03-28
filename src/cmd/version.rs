use std::fmt;

use semver::{Prerelease, Version};

use crate::{
    cli::VersionCommand,
    cmd::Command,
    conventional::{
        config::{Config, Increment, Type},
        CommitParser,
    },
    git::{GitHelper, VersionAndTag},
    semver::SemVer,
    Error,
};

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
    /// returns the versions under the given rev
    fn find_last_version(&self) -> Result<Option<VersionAndTag>, Error> {
        let prefix = self.prefix.as_str();
        let ignore_prereleases = // When bumping always ignore prereleases
            self.bump ||
            self.ignore_prereleases;
        Ok(GitHelper::new(prefix)?.find_last_version(self.rev.as_str(), ignore_prereleases)?)
    }

    /// Find the bump version based on the conventional commit types.
    ///
    /// - `fix` type commits are translated to PATCH releases.
    /// - `feat` type commits are translated to MINOR releases.
    /// - Commits with `BREAKING CHANGE` in the commits, regardless of type, are translated to MAJOR releases.
    ///
    /// If the project is in major version zero (0.y.z) the rules are:
    ///
    /// - `fix` type commits are translated to PATCH releases.
    /// - `feat` type commits are translated to PATCH releases.
    /// - Commits with `BREAKING CHANGE` in the commits, regardless of type, are translated to MINOR releases.
    fn find_bump_version(
        &self,
        last_v_tag: &str,
        mut last_version: SemVer,
        parser: &CommitParser,
        types: Vec<Type>,
        ignore_paths: &[std::path::PathBuf],
    ) -> Result<(Version, Label, String), Error> {
        let prefix = self.prefix.as_str();
        let git = GitHelper::new(prefix)?;
        let mut revwalk = git.revwalk()?;
        revwalk.push_range(format!("{}..{}", last_v_tag, self.rev).as_str())?;
        let i = revwalk
            .flatten()
            .filter_map(|oid| git.find_commit(oid).ok())
            .filter(|commit| git.commit_updates_relevant_paths(commit, &self.paths, ignore_paths))
            .filter_map(|commit| {
                let commit_sha = commit.id().to_string();

                commit
                    .message()
                    .and_then(|msg| parser.parse(msg).map(|c| (commit_sha, c)).ok())
            });

        let mut major = false;
        let mut minor = false;
        let mut patch = false;

        let major_version_zero = last_version.major() == 0;
        let mut commit_sha = None;
        for (sha, commit) in i {
            if commit_sha.is_none() {
                commit_sha = Some(sha);
            }
            if commit.is_breaking() {
                if major_version_zero {
                    minor = true;
                } else {
                    major = true;
                }
                break;
            }

            let option_commit_type = types.iter().find(|x| x.to_string() == commit.r#type);

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
            self.calc_prerelease(&mut last_version, &self.prerelease, &git, &commit_sha);
        }
        Ok((last_version.0, label, commit_sha))
    }

    fn calc_prerelease(
        &self,
        last_version: &mut SemVer,
        prerelease: &Prerelease,
        git: &GitHelper,
        commit_sha: &String,
    ) {
        if let Some(prerelease) =
            git.find_matching_prerelease(&*last_version, &prerelease, commit_sha)
        {
            last_version.0.pre = prerelease;
        } else {
            if let Some(prerelease) = git.find_last_prerelease(&*last_version, &prerelease) {
                last_version.0.pre = prerelease;
            }
            last_version.increment_prerelease(&prerelease);
        }
    }

    fn get_version(
        &self,
        scope_regex: String,
        strip_regex: String,
        types: Vec<Type>,
        initial_bump_version: Version,
        ignore_paths: &[std::path::PathBuf],
    ) -> Result<(Version, Label, String), Error> {
        if let Some(VersionAndTag {
            tag,
            mut version,
            commit_sha,
        }) = self.find_last_version()?
        {
            let v = if self.major {
                version.increment_major();
                (version.0, Label::Major, commit_sha)
            } else if self.minor {
                version.increment_minor();
                (version.0, Label::Minor, commit_sha)
            } else if self.patch {
                version.increment_patch();
                (version.0, Label::Patch, commit_sha)
            } else if self.bump {
                if version.is_prerelease() {
                    if self.prerelease.is_empty() {
                        version.pre_clear();
                        version.build_clear();
                        (version.0, Label::Release, commit_sha)
                    } else {
                        version.increment_prerelease(&self.prerelease);
                        (version.0, Label::Prerelease, commit_sha)
                    }
                } else {
                    let parser = CommitParser::builder()
                        .scope_regex(scope_regex)
                        .strip_regex(strip_regex)
                        .build();
                    self.find_bump_version(tag.as_str(), version, &parser, types, ignore_paths)?
                }
            } else {
                (version.0, Label::Release, commit_sha)
            };
            Ok(v)
        } else {
            let prefix = self.prefix.as_str();
            let git = GitHelper::new(prefix)?;
            let commit_sha = git.ref_to_commit(&self.rev)?;
            let commit_sha = commit_sha.id().to_string();
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
                    let mut initial_bump_version = SemVer(initial_bump_version);
                    self.calc_prerelease(
                        &mut initial_bump_version,
                        &self.prerelease,
                        &git,
                        &commit_sha,
                    );
                    Ok((initial_bump_version.0, Label::Prerelease, commit_sha))
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
    }
}

impl Command for VersionCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let ignore_paths = if self.ignore_paths.is_empty() {
            config.ignore_paths.clone()
        } else {
            self.ignore_paths.clone()
        };
        let initial_bump_version = self
            .initial_bump_version
            .clone()
            .unwrap_or(config.initial_bump_version);
        let (version, label, commit_sha) = self.get_version(
            config.scope_regex,
            config.strip_regex,
            config.types,
            initial_bump_version,
            &ignore_paths,
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
