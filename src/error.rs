use std::io;

use handlebars::{RenderError, TemplateError};
use thiserror::Error;

use crate::conventional;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error(transparent)]
    Parser(#[from] conventional::ParseError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    SemVer(#[from] semver::Error),
    #[error("check error")]
    Check,
    #[error("wrong type: {wrong_type}")]
    Type { wrong_type: String },
    #[error("canceled by user")]
    CancelledByUser,
}
