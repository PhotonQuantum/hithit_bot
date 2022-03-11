use std::error::Error;

use maplit::hashset;
use teloxide::types::{Message, MessageEntity, MessageEntityKind};

use crate::error::Result;
use crate::segments::{Segment, Segments};

pub fn elaborate_error(err: impl Error) -> impl IntoIterator<Item = Segment> {
    [
        Segment {
            text: String::from("An error occurred while processing your template.\n"),
            kind: hashset!(MessageEntityKind::Bold),
        },
        Segment {
            text: err.to_string(),
            kind: hashset!(MessageEntityKind::Code),
        },
    ]
}

fn elaborate_input(
    text: &str,
    entities: &[MessageEntity],
    segments: &Segments,
) -> impl IntoIterator<Item = Segment> {
    [
        Segment {
            kind: hashset! {MessageEntityKind::Bold},
            text: String::from("Input:\n"),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Code},
            text: format!("{}\n{:#?}\n", text, entities),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Bold},
            text: String::from("Parsed:\n"),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Code},
            text: format!("{:#?}\n", segments),
        },
    ]
}

fn elaborate_output(segments: &Segments) -> impl IntoIterator<Item = Segment> {
    [
        Segment {
            kind: hashset! {MessageEntityKind::Bold},
            text: String::from("Rendered:\n"),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Code},
            text: format!("{:#?}\n", segments),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Bold},
            text: String::from("Output:\n"),
        },
        Segment {
            kind: hashset! {MessageEntityKind::Code},
            text: format!("{}\n{:#?}", segments.text(), segments.entities()),
        },
    ]
}

pub fn elaborate(update: &Message, output: Result<Segments>) -> Segments {
    let text = update.text().expect("must be text message");
    let entities = update.entities().expect("must be text message");
    let input = Segments::build(text, entities);

    let elaborated_input = elaborate_input(text, entities, &input).into_iter();

    match output {
        Ok(output) => elaborated_input.chain(elaborate_output(&output)).into(),
        Err(e) => elaborated_input.chain(elaborate_error(e)).into(),
    }
}
