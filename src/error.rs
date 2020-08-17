use handlebars::{RenderError, TemplateError, TemplateFileError};
use std::{fmt, io};
use url::ParseError;

#[derive(Debug)]
pub(crate) enum Error {
    Git(git2::Error),
    Io(io::Error),
    Template(TemplateError),
    TemplateFile(TemplateFileError),
    Render(RenderError),
    Url(ParseError),
    Check,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Git(ref e) => write!(f, "{}", e),
            Self::Io(ref e) => write!(f, "{}", e),
            Self::Template(ref e) => write!(f, "{}", e),
            Self::TemplateFile(ref e) => write!(f, "{}", e),
            Self::Render(ref e) => write!(f, "{}", e),
            Self::Url(ref e) => write!(f, "{}", e),
            Self::Check => write!(f, "check error"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::Git(ref e) => Some(e),
            Self::Io(ref e) => Some(e),
            Self::Template(ref e) => Some(e),
            Self::TemplateFile(ref e) => Some(e),
            Self::Render(ref e) => Some(e),
            Self::Url(ref e) => Some(e),
            Self::Check => None,
        }
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Self::Git(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<TemplateError> for Error {
    fn from(e: TemplateError) -> Self {
        Self::Template(e)
    }
}

impl From<TemplateFileError> for Error {
    fn from(e: TemplateFileError) -> Self {
        Self::TemplateFile(e)
    }
}

impl From<RenderError> for Error {
    fn from(e: RenderError) -> Self {
        Self::Render(e)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Self::Url(e)
    }
}
