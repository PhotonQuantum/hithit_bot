#[macro_use]
extern crate maplit;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate thiserror;

use std::collections::{HashSet, VecDeque};

use anyhow::Result;
use const_format::concatcp;
use teloxide::prelude::*;
use teloxide::types::{MessageEntityKind, User};

use segments::{Segment, Segments};
use ParseMode::*;
use ProcessError::*;

use crate::formatter::{parse_curly, parse_naive};
use crate::utils::add_exclaim_mark;
use crate::ProcessError::NoneError;

#[macro_use]
mod segments;
mod formatter;
mod utils;

const EXPLAIN_COMMAND: &str = "/explain";
const BOT_NAME: &str = "hithit_rs_bot";
//noinspection RsTypeCheck
const EXPLAIN_COMMAND_EXTENDED: &str = concatcp!(EXPLAIN_COMMAND, "@", BOT_NAME);

#[derive(Debug, Copy, Clone)]
enum ParseMode {
    NaiveMode,
    CurlyMode,
}

#[tokio::main]
async fn main() {
    run().await;
}

fn get_user(user: &User) -> Segment {
    Segment {
        text: if let Some(last_name) = &user.last_name {
            format!("{} {}", user.first_name, last_name)
        } else {
            user.first_name.clone()
        },
        kind: hashset!(MessageEntityKind::TextMention { user: user.clone() }),
    }
}

fn get_reply_user(message: &Message) -> Option<Segment> {
    Some(if let Some(reply_msg) = message.reply_to_message() {
        let user = reply_msg.from()?;
        get_user(user)
    } else {
        Segment {
            text: String::from("自己"),
            kind: hashset!(MessageEntityKind::TextMention {
                user: message.from()?.clone()
            }),
        }
    })
}

#[derive(Debug)]
enum ProcessError {
    NoneError,
    SomeError(anyhow::Error),
}

fn process_ctx(msg: &Message) -> Option<Result<Segments>> {
    let text = msg.text()?;
    let entities = msg.entities()?;
    let sender = get_user(msg.from()?);
    let receiver = get_reply_user(msg)?;

    if !text.starts_with('/') {
        None
    } else if text.starts_with(EXPLAIN_COMMAND_EXTENDED) {
        Segments::build(text, entities)
            .drain_head(EXPLAIN_COMMAND_EXTENDED.len() + 1)
            .map(|segments| (segments, true))
    } else if text.starts_with(EXPLAIN_COMMAND) {
        Segments::build(text, entities)
            .drain_head(EXPLAIN_COMMAND.len() + 1)
            .map(|segments| (segments, true))
    } else {
        text.chars().nth(1).and_then(|chr| {
            if chr.len_utf8() > 1 {
                Segments::build(text, entities)
                    .drain_head(1)
                    .map(|x| (x, true))
            } else if chr == '^' {
                Segments::build(text, entities)
                    .drain_head(2)
                    .map(|x| (x, true))
            } else {
                Segments::build(text, entities)
                    .drain_head(1)
                    .map(|x| (x, false))
            }
        })
    }
    .map(|(segments, try_naive)| {
        parse_curly(&segments)
            .map_err(SomeError)
            .and_then(|fmt| {
                if fmt.indexed_holes() > 0 || !fmt.named_holes().is_empty() {
                    Ok((fmt, CurlyMode))
                } else if try_naive {
                    parse_naive(&segments)
                        .map(|x| (x, NaiveMode))
                        .map_err(SomeError)
                } else {
                    Err(NoneError)
                }
            })
            .and_then(|(fmt, mode)| {
                fmt.format(
                    &[sender.clone(), receiver.clone()],
                    hashmap! {"sender".to_owned() => sender.clone(),
                    "receiver".to_owned() => receiver.clone(),
                    // the followings are suggested by @tonyxty
                    "penetrator".to_owned() => sender.clone(),
                    "1".to_owned() => sender.clone(),
                    "0".to_owned() => receiver.clone()},
                )
                .map_err(anyhow::Error::from)
                .map_err(SomeError)
                .map(Segments::trim)
                .map(add_exclaim_mark)
                .map(|mut segments| match mode {
                    NaiveMode => {
                        let inner = segments.inner_mut();
                        inner.push_front(empty_segment!());
                        inner.push_front(sender);
                        segments
                    }
                    CurlyMode => segments,
                })
            })
    })
    .and_then(|maybe_segments| match maybe_segments {
        Ok(segments) => Some(Ok(segments)),
        Err(ProcessError::NoneError) => None,
        Err(ProcessError::SomeError(err)) => Some(Err(err)),
    })
}

fn error_report<T: Into<anyhow::Error>>(err: T) -> VecDeque<Segment> {
    let mut deque = VecDeque::new();
    deque.push_back(Segment {
        text: String::from("An error occurred while processing your template.\n"),
        kind: hashset!(MessageEntityKind::Bold),
    });
    deque.push_back(Segment {
        text: err.into().to_string(),
        kind: hashset!(MessageEntityKind::Code),
    });
    deque
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting hithit bot");

    let bot = Bot::builder().build();

    teloxide::repl(bot, |msg| async move {
        log::debug!(
            "Income message: {:?} {:?}",
            msg.update.text(),
            msg.update.entities()
        );

        let reply = process_ctx(&msg.update)
            .map(|reply| {
                log::debug!("Reply: {:?}", reply);
                reply
            })
            .map(|reply| {
                let text = msg.update.text().unwrap();
                if text.starts_with(EXPLAIN_COMMAND) {
                    let orig_segments =
                        Segments::build(msg.update.text().unwrap(), msg.update.entities().unwrap());
                    let mut deque = VecDeque::new();
                    deque.push_back(Segment {
                        kind: hashset! {MessageEntityKind::Bold},
                        text: String::from("Input:\n"),
                    });
                    deque.push_back(Segment {
                        kind: hashset! {MessageEntityKind::Code},
                        text: format!("{:#?}\n", orig_segments),
                    });
                    match reply {
                        Ok(segments) => {
                            deque.push_back(Segment {
                                kind: hashset! {MessageEntityKind::Bold},
                                text: String::from("Rendered:\n"),
                            });
                            deque.push_back(Segment {
                                kind: hashset! {MessageEntityKind::Code},
                                text: format!("{:#?}", segments),
                            });
                            Segments::from(deque)
                        }
                        Err(err) => {
                            deque.extend(error_report(err));
                            Segments::from(deque)
                        }
                    }
                } else {
                    reply.unwrap_or_else(|err| error_report(err).into())
                }
            });

        match reply {
            None => {}
            Some(segments) => {
                msg.answer(segments.text())
                    .entities(segments.entities())
                    .send()
                    .await?;
            }
        };

        respond(())
    })
    .await;
}
