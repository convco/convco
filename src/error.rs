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
    GixRemoteFindExistingError(Box<gix::remote::find::existing::Error>),
    #[error(transparent)]
    #[cfg(feature = "gix")]
    GixOpenError(Box<gix::open::Error>),
    #[error(transparent)]
    #[cfg(feature = "gix")]
    GixDiscoverError(Box<gix::discover::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixReferenceIter(Box<gix::reference::iter::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixReferenceIterInet(Box<gix::reference::iter::init::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixRevisionSpecParseSingle(Box<gix::revision::spec::parse::single::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixRevisionWalkError(Box<gix::revision::walk::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixObjectFindExistingError(Box<gix::object::find::existing::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    Gix(Box<gix::object::peel::to_kind::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixCommitError(Box<gix::object::commit::Error>),
    #[cfg(feature = "gix")]
    #[error(transparent)]
    GixObjectDecodeError(Box<gix::objs::decode::Error>),
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
    Regex(#[from] regex::Error),
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

#[cfg(feature = "gix")]
macro_rules! impl_boxed_from {
    ($source:ty, $variant:ident) => {
        impl From<$source> for ConvcoError {
            fn from(value: $source) -> Self {
                Self::$variant(Box::new(value))
            }
        }
    };
}

#[cfg(feature = "gix")]
impl_boxed_from!(
    gix::remote::find::existing::Error,
    GixRemoteFindExistingError
);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::open::Error, GixOpenError);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::discover::Error, GixDiscoverError);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::reference::iter::Error, GixReferenceIter);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::reference::iter::init::Error, GixReferenceIterInet);
#[cfg(feature = "gix")]
impl_boxed_from!(
    gix::revision::spec::parse::single::Error,
    GixRevisionSpecParseSingle
);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::revision::walk::Error, GixRevisionWalkError);
#[cfg(feature = "gix")]
impl_boxed_from!(
    gix::object::find::existing::Error,
    GixObjectFindExistingError
);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::object::peel::to_kind::Error, Gix);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::object::commit::Error, GixCommitError);
#[cfg(feature = "gix")]
impl_boxed_from!(gix::objs::decode::Error, GixObjectDecodeError);
