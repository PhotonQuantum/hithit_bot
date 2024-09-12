use thiserror::Error;

pub use crate::parser::ParseError as Parse;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Format {
    #[error("index {0} not found")]
    InvalidIndex(usize),
    #[error("key {0} not found")]
    InvalidKey(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("this message is not intended to be handled by this bot")]
    ShouldNotHandle,
    #[error("format error: {0}")]
    Format(#[from] Format),
    #[error("parse error: {0}")]
    Parse(#[from] Box<Parse>),
}

impl From<Parse> for Error {
    fn from(value: Parse) -> Self {
        Self::Parse(Box::new(value))
    }
}

pub trait ErrorExt<T> {
    fn lift_should_not_handle(self) -> Result<T, Error>;
}

impl<T> ErrorExt<Self> for Result<T, Error> {
    fn lift_should_not_handle(self) -> Result<Self, Error> {
        match self {
            Ok(t) => Ok(Ok(t)),
            Err(Error::ShouldNotHandle) => Err(Error::ShouldNotHandle),
            Err(e) => Ok(Err(e)),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExportedError {
    #[error("format error: {0}")]
    Format(#[from] Format),
    #[error("parse error: {0}")]
    Parse(#[from] Box<Parse>),
}

impl From<Parse> for ExportedError {
    fn from(value: Parse) -> Self {
        Self::Parse(Box::new(value))
    }
}
