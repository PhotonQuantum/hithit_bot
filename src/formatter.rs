use std::collections::{HashMap, HashSet, VecDeque};

use maplit::hashmap;
use teloxide::types::MessageEntityKind;

use crate::error::Format as FormatError;
use crate::segments::{Segment, Segments};

const TERMINATION_MARKS: [char; 11] = ['。', '，', '！', '？', '；', '、', '.', ',', '!', '?', ';'];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HoleIdent {
    Anonymous,
    Indexed(usize),
    Named(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Token {
    Segment(Segment),
    Hole {
        kind: HashSet<MessageEntityKind>,
        ident: HoleIdent,
    },
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct FormatContext {
    indexed_args: Vec<Segment>,
    named_args: HashMap<&'static str, Segment>,
}

impl FormatContext {
    pub fn new(sender: Segment, receiver: Segment, me: Segment) -> Self {
        Self {
            indexed_args: vec![sender.clone(), receiver.clone()],
            named_args: hashmap! {
                "sender" => sender.clone(),
                "receiver" => receiver,
                "penetrator" => sender,  // suggested by @tonyxty
                "self" => me.clone(),
                "me" => me.clone(),
                "this" => me
            },
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Formatter {
    pub data: Vec<Token>,
    pub indexed: usize,
    pub named: HashSet<String>,
}

impl Formatter {
    pub const fn indexed_holes(&self) -> usize {
        self.indexed
    }
    pub const fn named_holes(&self) -> &HashSet<String> {
        &self.named
    }
    pub fn format(&self, ctx: &FormatContext) -> Result<Segments, FormatError> {
        self.data
            .iter()
            .map(fill_placeholder(ctx))
            .collect::<Result<VecDeque<_>, _>>()
            .map(Segments::new)
            .map(Segments::trim)
            .map(add_exclaim_mark)
    }
}

fn fill_placeholder(
    ctx: &FormatContext,
) -> impl FnMut(&Token) -> Result<Segment, FormatError> + '_ {
    let mut implicit_idx: usize = 0;
    move |token| match token {
        Token::Segment(segment) => Ok(segment.clone()),
        Token::Hole { kind, ident } => match ident {
            HoleIdent::Anonymous => ctx
                .indexed_args
                .get(implicit_idx)
                .ok_or(FormatError::InvalidIndex(implicit_idx))
                .map(|v| {
                    implicit_idx += 1;
                    v
                }),
            HoleIdent::Indexed(idx) => ctx
                .indexed_args
                .get(*idx)
                .ok_or(FormatError::InvalidIndex(*idx)),
            HoleIdent::Named(name) => ctx
                .named_args
                .get(name.as_str())
                .ok_or_else(|| FormatError::InvalidKey(name.clone())),
        }
        .map(|segment_to_merge| Segment {
            kind: kind
                .clone()
                .union(&segment_to_merge.kind)
                .cloned()
                .collect(),
            text: segment_to_merge.text.clone(),
        }),
    }
}

fn end_with_marks(input: &str) -> bool {
    TERMINATION_MARKS
        .iter()
        .any(|chr| input.chars().last().map_or(false, |ref end| end == chr))
}

fn add_exclaim_mark(mut input: Segments) -> Segments {
    if let Some(segment) = input.back() {
        if !end_with_marks(segment.text.as_str()) {
            input.push_back(Segment {
                text: String::from("！"),
                kind: HashSet::new(),
            });
            return input;
        }
    }
    input
}
