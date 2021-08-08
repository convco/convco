use std::str::FromStr;

use semver::{BuildMetadata, Prerelease, Version};

use crate::error::Error;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SemVer(pub(crate) Version);

impl SemVer {
    pub fn increment_patch(&mut self) {
        self.0.patch += 1;
        self.0.pre = Prerelease::EMPTY;
        self.0.build = BuildMetadata::EMPTY;
    }

    pub fn increment_minor(&mut self) {
        self.0.minor += 1;
        self.0.patch = 0;
        self.0.pre = Prerelease::EMPTY;
        self.0.build = BuildMetadata::EMPTY;
    }

    pub fn increment_major(&mut self) {
        self.0.major += 1;
        self.0.minor = 0;
        self.0.patch = 0;
        self.0.pre = Prerelease::EMPTY;
        self.0.build = BuildMetadata::EMPTY;
    }

    pub fn major(&self) -> u64 {
        self.0.major
    }

    pub fn patch(&self) -> u64 {
        self.0.patch
    }

    pub fn is_prerelease(&self) -> bool {
        self.0.pre.is_empty()
    }

    pub fn pre_clear(&mut self) {
        self.0.pre = Prerelease::EMPTY
    }

    pub fn build_clear(&mut self) {
        self.0.build = BuildMetadata::EMPTY
    }
}

impl FromStr for SemVer {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Version::from_str(s)?))
    }
}
