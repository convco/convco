use std::{io, process::ExitStatus};

use handlebars::{RenderError, TemplateError};
use thiserror::Error;

use crate::conventional;

#[derive(Debug, Error)]
pub enum ConvcoError {
    #[error(transparent)]
    Dialoguer(#[from] dialoguer::Error),
    #[cfg(feature = "git2")]
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixRemoteFindExistingError(#[from] gix::remote::find::existing::Error),
    #[error(transparent)]
    #[cfg(feature = "gix")]
    GixOpenError(#[from] gix::open::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixReferenceIter(#[from] gix::reference::iter::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixReferenceIterInet(#[from] gix::reference::iter::init::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixRevisionSpecParseSingle(#[from] gix::revision::spec::parse::single::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixRevisionWalkError(#[from] gix::revision::walk::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixObjectFindExistingError(#[from] gix::object::find::existing::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    Gix(#[from] gix::object::peel::to_kind::Error),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixCommitError(#[from] gix::object::commit::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Template(#[from] Box<TemplateError>),
    #[error(transparent)]
    Parser(#[from] conventional::ParseError),
    #[error(transparent)]
    Render(#[from] Box<RenderError>),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    SemVer(#[from] semver::Error),
    #[error(transparent)]
    Yaml(#[from] serde_norway::Error),
    #[error(transparent)]
    Utf8(#[from] bstr::Utf8Error),
    #[error(transparent)]
    Jiff(#[from] jiff::Error),
    #[error("check error")]
    Check,
    #[error("wrong type: {wrong_type}")]
    Type { wrong_type: String },
    #[error("canceled by user")]
    CancelledByUser,
    #[error("git commit failed: {0}")]
    GitCommitFailed(ExitStatus),
}
