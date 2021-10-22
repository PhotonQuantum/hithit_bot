use std::collections::HashSet;
use std::iter::FromIterator;

use crate::formatter::{Formatter, HoleIdent, Token};
use crate::segments::{Segment, Segments};

pub fn parse(segments: &Segments) -> Formatter {
    let mut bypass = false;

    let mut data: Vec<Token> = vec![
        Token::Hole {
            kind: HashSet::new(),
            ident: HoleIdent::Named(String::from("sender")),
        },
        Token::Segment(Segment::empty()),
    ]
    .into_iter()
    .chain(segments.iter().flat_map(|segment| {
        if bypass {
            return vec![Token::Segment(segment.clone())];
        }

        let chars: Vec<_> = segment.text.chars().collect();
        chars.iter().position(|c| c.is_whitespace()).map_or_else(
            || vec![Token::Segment(segment.clone())],
            |offset| {
                bypass = true;
                vec![
                    Token::Segment(Segment {
                        kind: segment.kind.clone(),
                        text: format!("{} ", String::from_iter(&chars[..offset])),
                    }),
                    Token::Hole {
                        kind: segment.kind.clone(),
                        ident: HoleIdent::Indexed(1),
                    },
                    Token::Segment(Segment {
                        kind: segment.kind.clone(),
                        text: format!(" {}", String::from_iter(&chars[offset + 1..])),
                    }),
                ]
            },
        )
    }))
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
