mod conventional;
mod error;
mod git;
pub mod strip;

pub use conventional::{
    changelog,
    commit::{Footer, FooterKey},
    config::{Increment, Type},
    CommitParser, Config, ParseError,
};
pub use error::ConvcoError;
pub use git::{
    open_repo, Commit, CommitTrait, MaxMajorsIterExt, MaxMinorsIterExt, MaxPatchesIterExt, Repo,
    RevWalkOptions,
};
