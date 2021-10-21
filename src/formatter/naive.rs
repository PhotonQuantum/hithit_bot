use std::collections::HashSet;

use crate::formatter::HoleIdent;
use crate::segments::{Segment, Segments};

use super::{Formatter, Token};

pub fn parse(segments: &Segments) -> Formatter {
    let mut bypass = false;

    let mut data: Vec<Token> = segments
        .inner_ref()
        .iter()
        .flat_map(|segment| {
            if bypass {
                vec![Token::Segment(segment.clone())]
            } else if let Some(offset) = segment.text.chars().position(char::is_whitespace) {
                let mut chars_iter = segment.text.chars();
                bypass = true;
                vec![
                    Token::Segment(Segment {
                        kind: segment.kind.clone(),
                        text: format!("{} ", chars_iter.by_ref().take(offset).collect::<String>()),
                    }),
                    Token::Hole {
                        kind: segment.kind.clone(),
                        ident: HoleIdent::Indexed(1),
                    },
                    Token::Segment(Segment {
                        kind: segment.kind.clone(),
                        text: format!(" {}", chars_iter.skip(1).collect::<String>()),
                    }),
                ]
            } else {
                vec![Token::Segment(segment.clone())]
            }
        })
        .collect();

    if bypass {
        Formatter {
            data,
            indexed: 1,
            named: HashSet::new(),
        }
    } else {
        data.push(Token::Segment(Segment {
            text: String::from(
                if segments
                    .inner_ref()
                    .back()
                    .map_or(false, |segment| segment.text.ends_with('了'))
                {
                    " "
                } else {
                    "了 "
                },
            ),
            kind: HashSet::new(),
        }));
        data.push(Token::Hole {
            kind: HashSet::new(),
            ident: HoleIdent::Indexed(1),
        });
        Formatter {
            data,
            indexed: 0,
            named: HashSet::new(),
        }
    }
}
