use std::fmt;

use semver::Version;

use crate::{
    cli::VersionCommand,
    cmd::Command,
    conventional::{CommitParser, Config, Type},
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
        Ok(GitHelper::new(prefix)?.find_last_version(self.rev.as_str())?)
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
    ) -> Result<(Version, Label), Error> {
        let prefix = self.prefix.as_str();
        let git = GitHelper::new(prefix)?;
        let mut revwalk = git.revwalk()?;
        revwalk.push_range(format!("{}..{}", last_v_tag, self.rev).as_str())?;
        let i = revwalk
            .flatten()
            .filter_map(|oid| git.find_commit(oid).ok())
            .filter(|commit| git.commit_updates_any_path(commit, &self.paths))
            .filter_map(|commit| commit.message().and_then(|msg| parser.parse(msg).ok()));

        let mut major = false;
        let mut minor = false;
        let mut patch = false;

        let major_version_zero = last_version.major() == 0;

        for commit in i {
            if commit.is_breaking() {
                if major_version_zero {
                    minor = true;
                } else {
                    major = true;
                }
                break;
            }
            match (commit.r#type, major_version_zero) {
                (Type::Feat, true) => patch = true,
                (Type::Feat, false) => minor = true,
                (Type::Fix, _) => patch = true,
                _ => (),
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
        if !self.prerelease.is_empty() {
            last_version.increment_prerelease(&self.prerelease);
        }
        Ok((last_version.0, label))
    }

    fn get_version(&self, scope_regex: String) -> Result<(Version, Label), Error> {
        if let Some(VersionAndTag { tag, mut version }) = self.find_last_version()? {
            let v = if self.major {
                version.increment_major();
                (version.0, Label::Major)
            } else if self.minor {
                version.increment_minor();
                (version.0, Label::Minor)
            } else if self.patch {
                version.increment_patch();
                (version.0, Label::Patch)
            } else if self.bump {
                if version.is_prerelease() {
                    if self.prerelease.is_empty() {
                        version.pre_clear();
                        version.build_clear();
                        (version.0, Label::Release)
                    } else {
                        version.increment_prerelease(&self.prerelease);
                        (version.0, Label::Prerelease)
                    }
                } else {
                    let parser = CommitParser::builder().scope_regex(scope_regex).build();
                    self.find_bump_version(tag.as_str(), version, &parser)?
                }
            } else {
                (version.0, Label::Release)
            };
            Ok(v)
        } else if self.bump || self.minor {
            Ok(("0.1.0".parse()?, Label::Minor))
        } else if self.major {
            Ok(("1.0.0".parse()?, Label::Major))
        } else if self.patch {
            Ok(("0.0.1".parse()?, Label::Patch))
        } else {
            Ok(("0.0.0".parse()?, Label::Patch))
        }
    }
}

impl Command for VersionCommand {
    fn exec(&self, config: Config) -> anyhow::Result<()> {
        let (version, label) = self.get_version(config.scope_regex)?;
        if self.label {
            println!("{label}");
        } else {
            println!("{version}");
        }
        Ok(())
    }
}
