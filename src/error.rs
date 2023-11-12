use std::{io, process::ExitStatus};

use handlebars::{RenderError, TemplateError};
use thiserror::Error;

use crate::conventional;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Dialoguer(#[from] dialoguer::Error),
    #[error(transparent)]
    Git(#[from] git2::Error),
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
    Yaml(#[from] serde_yaml::Error),
    #[error("check error")]
    Check,
    #[error("wrong type: {wrong_type}")]
    Type { wrong_type: String },
    #[error("canceled by user")]
    CancelledByUser,
    #[error("git commit failed: {0}")]
    GitCommitFailed(ExitStatus),
}
