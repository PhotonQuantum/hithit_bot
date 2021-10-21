use std::collections::HashSet;

use crate::segments::{Segment, Segments};

const TERMINATION_MARKS: [char; 11] = ['。', '，', '！', '？', '；', '、', '.', ',', '!', '?', ';'];

pub fn end_with_marks(input: &str) -> bool {
    TERMINATION_MARKS
        .iter()
        .any(|chr| input.chars().last().map_or(false, |ref end| end == chr))
}

pub fn add_exclaim_mark(mut input: Segments) -> Segments {
    if let Some(segment) = input.inner_ref().back() {
        if !end_with_marks(segment.text.as_str()) {
            input.inner_mut().push_back(Segment {
                text: String::from("！"),
                kind: HashSet::new(),
            });
            return input;
        }
    }
    input
}
