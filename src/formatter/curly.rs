use std::cmp::max;
use std::collections::HashSet;

use anyhow::Result;
use pest::Parser;

use crate::segments::{Segment, Segments};

use super::{Formatter, HoleIdent, Token};

#[derive(Parser)]
#[grammar = "fmt.pest"]
struct FmtParser;

pub fn parse_curly(segments: &Segments) -> Result<Formatter> {
    let mut ast = vec![];
    let mut anonymous_counter = 0;
    let mut max_indexed = 0;
    let mut named: HashSet<String> = HashSet::new();

    for segment in segments.inner_ref() {
        let mut pairs = FmtParser::parse(Rule::formatter, &segment.text)?;
        let pair = pairs.next().unwrap();

        match pair.as_rule() {
            Rule::formatter => {
                let mut buffer: String = String::new();

                let pairs = pair.into_inner();
                for pair in pairs {
                    match pair.as_rule() {
                        Rule::ident => {
                            if !buffer.is_empty() {
                                ast.push(Token::Segment(Segment {
                                    kind: segment.kind.clone(),
                                    text: buffer.drain(..).collect(),
                                }));
                            };
                            ast.push(Token::Hole {
                                kind: segment.kind.clone(),
                                ident: {
                                    let ident: &str = pair.as_str();
                                    if ident.is_empty() {
                                        anonymous_counter += 1;
                                        HoleIdent::Anonymous
                                    } else if let Ok(idx) = ident.parse::<usize>() {
                                        max_indexed = max(max_indexed, idx + 1);
                                        HoleIdent::Indexed(idx)
                                    } else {
                                        named.insert(ident.to_string());
                                        HoleIdent::Named(ident.to_string())
                                    }
                                },
                            })
                        }
                        Rule::char => buffer.push_str(pair.as_str()),
                        Rule::escaped => {
                            buffer.push(match pair.as_str() {
                                "{{" => '{',
                                "}}" => '}',
                                _ => unreachable!(),
                            });
                        }
                        Rule::EOI => {}
                        _ => unreachable!(),
                    }
                }
                if !buffer.is_empty() {
                    ast.push(Token::Segment(Segment {
                        kind: segment.kind.clone(),
                        text: buffer.drain(..).collect(),
                    }));
                };
            }
            Rule::EOI => {}
            _ => unreachable!(),
        };
    }

    Ok(Formatter {
        data: ast,
        indexed: max(anonymous_counter, max_indexed),
        named,
    })
}
