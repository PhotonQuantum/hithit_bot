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
    #[error("this messaage is not intended to be handled by this bot")]
    ShouldNotHandle,
    #[error("format error: {0}")]
    Format(#[from] Format),
    #[error("parse error: {0}")]
    Parse(#[from] Parse),
}
