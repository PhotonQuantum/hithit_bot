use crate::error::{Error, Result};
use crate::formatter::Formatter;
use crate::segments::Segments;

pub type ParseError = curly::Error;

mod curly;
mod naive;

pub struct Parser {
    input: Segments,
    try_naive: bool,
}

impl Parser {
    pub const fn new(input: Segments, try_naive: bool) -> Self {
        Self { input, try_naive }
    }
}

impl Parser {
    pub fn try_as_formatter(&self) -> Result<Formatter> {
        let with_curly = curly::parse(&self.input)?;

        if with_curly.indexed_holes() > 0 || !with_curly.named_holes().is_empty() {
            Ok(with_curly)
        } else if self.try_naive {
            Ok(naive::parse(&self.input))
        } else {
            Err(Error::ShouldNotHandle)
        }
    }
}
