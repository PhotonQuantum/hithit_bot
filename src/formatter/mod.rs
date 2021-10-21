use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use teloxide::types::MessageEntityKind;

pub use curly::parse as parse_curly;
pub use naive::parse as parse_naive;

use crate::segments::{Segment, Segments};

mod curly;
mod naive;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("index {0} not found")]
    InvalidIndex(usize),
    #[error("key {0} not found")]
    InvalidKey(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HoleIdent {
    Anonymous,
    Indexed(usize),
    Named(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Token {
    Segment(Segment),
    Hole {
        kind: HashSet<MessageEntityKind>,
        ident: HoleIdent,
    },
}

#[derive(Debug, Default, Clone)]
pub struct Formatter {
    data: Vec<Token>,
    indexed: usize,
    named: HashSet<String>,
}

impl Formatter {
    pub const fn indexed_holes(&self) -> usize {
        self.indexed
    }
    pub const fn named_holes(&self) -> &HashSet<String> {
        &self.named
    }
    pub fn format(
        &self,
        indexed_args: &[Segment],
        named_args: &HashMap<&str, Segment>,
    ) -> Result<Segments, FormatError> {
        let mut implicit_idx: usize = 0;

        Ok(self
            .data
            .iter()
            .map(|token| match token {
                Token::Segment(segment) => Ok(segment.clone()),
                Token::Hole { kind, ident } => match ident {
                    HoleIdent::Anonymous => indexed_args.get(implicit_idx).map_or(
                        Err(FormatError::InvalidIndex(implicit_idx)),
                        |value| {
                            implicit_idx += 1;
                            Ok(Segment {
                                kind: kind.clone().union(&value.kind).cloned().collect(),
                                text: value.text.clone(),
                            })
                        },
                    ),
                    HoleIdent::Indexed(idx) => indexed_args.get(*idx).map_or_else(
                        || Err(FormatError::InvalidIndex(*idx)),
                        |value| {
                            Ok(Segment {
                                kind: kind.clone().union(&value.kind).cloned().collect(),
                                text: value.text.clone(),
                            })
                        },
                    ),
                    HoleIdent::Named(name) => named_args.get(name.as_str()).map_or_else(
                        || Err(FormatError::InvalidKey(name.clone())),
                        |value| {
                            Ok(Segment {
                                kind: kind.clone().union(&value.kind).cloned().collect(),
                                text: value.text.clone(),
                            })
                        },
                    ),
                },
            })
            .collect::<Result<VecDeque<_>, _>>()?
            .into())
    }
}
