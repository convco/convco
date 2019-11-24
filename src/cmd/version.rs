use crate::{
    cli::VersionCommand,
    cmd::Command,
    conventional::{Commit, Type},
    git::{GitHelper, VersionAndTag},
    Error,
};
use semver::Version;
use std::str::FromStr;

impl VersionCommand {
    /// returns the versions under the given rev
    fn find_last_version(&self) -> Result<Option<VersionAndTag>, Error> {
        let prefix = self.prefix.as_str();
        Ok(GitHelper::new(prefix)?.find_last_version(self.rev.as_str())?)
    }

    fn find_bump_version(
        &self,
        last_v_tag: &str,
        mut last_version: Version,
    ) -> Result<Version, Error> {
        let prefix = self.prefix.as_str();
        let git = GitHelper::new(prefix)?;
        let mut revwalk = git.revwalk()?;
        revwalk.push_range(format!("{}..{}", last_v_tag, self.rev).as_str())?;
        let i = revwalk
            .flatten()
            .filter_map(|oid| git.find_commit(oid).ok())
            .filter_map(|commit| commit.message().and_then(|msg| Commit::from_str(msg).ok()));

        let mut major = false;
        let mut minor = false;
        let mut patch = false;

        for commit in i {
            if commit.breaking {
                major = true;
                break;
            }
            match commit.r#type {
                Type::Feat => {
                    minor = true;
                }
                Type::Fix => patch = true,
                _ => (),
            }
        }
        match (major, minor, patch) {
            (true, _, _) => last_version.increment_major(),
            (false, true, _) => last_version.increment_minor(),
            (false, false, true) => last_version.increment_patch(),
            // TODO what should be the behaviour? always increment patch? or stay on same version?
            _ => (),
        }
        Ok(last_version)
    }
}

impl Command for VersionCommand {
    fn exec(&self) -> Result<(), Error> {
        if let Some(VersionAndTag { tag, mut version }) = self.find_last_version()? {
            let v = if self.bump {
                if version.is_prerelease() {
                    version.pre.clear();
                    version.build.clear();
                    version
                } else {
                    self.find_bump_version(tag.as_str(), version)?
                }
            } else if self.major {
                version.increment_major();
                version
            } else if self.minor {
                version.increment_minor();
                version
            } else if self.patch {
                version.increment_patch();
                version
            } else {
                version
            };
            println!("{}", v);
        } else {
            println!("0.1.0");
        }
        Ok(())
    }
}
