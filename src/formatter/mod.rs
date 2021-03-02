use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use teloxide::types::MessageEntityKind;

pub use curly::parse_curly;
pub use naive::parse_naive;
use FormatError::*;

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
    // Indexed(usize),
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
    pub fn indexed_holes(&self) -> usize {
        self.indexed
    }
    pub fn named_holes(&self) -> &HashSet<String> {
        &self.named
    }
    pub fn format(
        &self,
        indexed_args: &[Segment],
        named_args: HashMap<String, Segment>,
    ) -> Result<Segments, FormatError> {
        let mut implicit_idx: usize = 0;

        Ok(self
            .data
            .iter()
            .map(|token| match token {
                Token::Segment(segment) => Ok(segment.clone()),
                Token::Hole { kind, ident } => match ident {
                    HoleIdent::Anonymous => {
                        if let Some(value) = indexed_args.get(implicit_idx) {
                            implicit_idx += 1;
                            Ok(Segment {
                                kind: kind.clone().union(&value.kind).cloned().collect(),
                                text: value.text.clone(),
                            })
                        } else {
                            Err(InvalidIndex(implicit_idx))
                        }
                    }
                    // HoleIdent::Indexed(idx) => {
                    //     if let Some(value) = indexed_args.get(*idx) {
                    //         Ok(Segment {
                    //             kind: kind.clone().union(&value.kind).cloned().collect(),
                    //             text: value.text.clone(),
                    //         })
                    //     } else {
                    //         Err(InvalidIndex(*idx))
                    //     }
                    // }
                    HoleIdent::Named(name) => {
                        if let Some(value) = named_args.get(name) {
                            Ok(Segment {
                                kind: kind.clone().union(&value.kind).cloned().collect(),
                                text: value.text.clone(),
                            })
                        } else {
                            Err(InvalidKey(name.clone()))
                        }
                    }
                },
            })
            .collect::<Result<VecDeque<_>, _>>()?
            .into())
    }
}
