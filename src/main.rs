#[macro_use]
extern crate maplit;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate thiserror;

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use const_format::concatcp;
use lru_cache::LruCache;
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct MessageMeta {
    chat_id: i64,
    message_id: i32,
    sender: User,
}

impl MessageMeta {
    pub fn from_message(msg: &Message) -> Self {
        MessageMeta {
            chat_id: msg.chat.id,
            message_id: msg.id,
            sender: msg.from().unwrap().clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    run().await;
}

fn get_reply_user(
    bot_user: &User,
    rev_sent_msg: &mut LruCache<MessageMeta, MessageMeta>,
    message: &Message,
) -> Option<Segment> {
    if let Some(reply_msg) = message.reply_to_message() {
        let user = reply_msg.from()?;
        if user == bot_user {
            let test = rev_sent_msg.get_mut(&MessageMeta::from_message(reply_msg));
            test.map(|msg| {
                let sender = &msg.sender;
                message
                    .from()
                    .map(|curr_sender| {
                        if sender == curr_sender {
                            Segment::from_user_with_name(user, String::from("自己"))
                        } else {
                            sender.into()
                        }
                    })
                    .unwrap_or_else(|| sender.into())
            })
            .unwrap_or_else(|| user.into())
        } else {
            user.into()
        }
    } else {
        let user = message.from()?;
        Segment::from_user_with_name(user, String::from("自己"))
    }
    .into()
}

#[derive(Debug)]
enum ProcessError {
    NoneError,
    SomeError(anyhow::Error),
}

fn process_ctx(
    bot_user: &User,
    rev_sent_msg: &mut LruCache<MessageMeta, MessageMeta>,
    msg: &Message,
) -> Option<Result<Segments>> {
    let text = msg.text()?;
    let entities = msg.entities()?;
    let sender = Segment::from_user(msg.from()?);
    let me = Segment::from_user_with_name(msg.from()?, String::from("自己"));
    let receiver = get_reply_user(bot_user, rev_sent_msg, msg)?;

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
                    hashmap! {"sender" => sender.clone(),
                    "receiver" => receiver.clone(),
                    "penetrator" => sender.clone(),  // suggested by @tonyxty
                    "self" => me.clone(),
                    "me" => me.clone(),
                    "this" => me.clone()
                    },
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

fn elaborate(update: &Message, reply: Result<Segments>) -> Segments {
    let text = update.text().unwrap();
    let entities = update.entities().unwrap();
    let orig_segments = Segments::build(text, entities);
    let mut deque = VecDeque::new();
    deque.push_back(Segment {
        kind: hashset! {MessageEntityKind::Bold},
        text: String::from("Input:\n"),
    });
    deque.push_back(Segment {
        kind: hashset! {MessageEntityKind::Code},
        text: format!("{}\n{:#?}\n", text, entities),
    });
    deque.push_back(Segment {
        kind: hashset! {MessageEntityKind::Bold},
        text: String::from("Parsed:\n"),
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
                text: format!("{:#?}\n", segments),
            });
            deque.push_back(Segment {
                kind: hashset! {MessageEntityKind::Bold},
                text: String::from("Output:\n"),
            });
            deque.push_back(Segment {
                kind: hashset! {MessageEntityKind::Code},
                text: format!("{}\n{:#?}", segments.text(), segments.entities()),
            });
            Segments::from(deque)
        }
        Err(err) => {
            deque.extend(error_report(err));
            Segments::from(deque)
        }
    }
}

async fn run() {
    teloxide::enable_logging!();
    log::warn!("Starting hithit bot");

    let bot = Bot::builder().build();

    let sent_map = Arc::new(Mutex::new(LruCache::new(8192)));
    let sent_map_new = sent_map.clone();
    let sent_map_edited = sent_map.clone();

    let rev_sent_map = Arc::new(Mutex::new(LruCache::new(8192)));
    let rev_sent_map_new = rev_sent_map.clone();
    let rev_sent_map_edited = rev_sent_map.clone();

    Dispatcher::new(bot)
        .messages_handler(move |rx: DispatcherHandlerRx<Message>| {
            rx.for_each(move |upd| {
                let sent_map_new = sent_map_new.clone();
                let rev_sent_map_new = rev_sent_map_new.clone();
                async move {
                    let update = &upd.update;
                    let maybe_me = upd.bot.get_me().send().await;
                    if let Ok(me) = maybe_me {
                        let me = &me.user;
                        let reply = process_ctx(me, &mut *rev_sent_map_new.lock().unwrap(), update)
                            .map(|reply| {
                                let text = update.text().unwrap();
                                if text.starts_with(EXPLAIN_COMMAND) {
                                    elaborate(update, reply)
                                } else {
                                    reply.unwrap_or_else(|err| error_report(err).into())
                                }
                            });

                        match reply {
                            None => {}
                            Some(segments) => {
                                let text = segments.text();
                                let entities = segments.entities();

                                let sent_reply = upd.answer(text).entities(entities).send().await;

                                if let Ok(sent_reply) = sent_reply {
                                    let mut sent_map = sent_map_new.lock().unwrap();
                                    sent_map.insert(
                                        MessageMeta::from_message(update),
                                        MessageMeta::from_message(&sent_reply),
                                    );
                                    let mut rev_sent_map = rev_sent_map_new.lock().unwrap();
                                    rev_sent_map.insert(
                                        MessageMeta::from_message(&sent_reply),
                                        MessageMeta::from_message(update),
                                    );
                                }
                            }
                        };
                    }
                }
            })
        })
        .edited_messages_handler(move |rx: DispatcherHandlerRx<Message>| {
            rx.for_each(move |upd| {
                let sent_map_edited = sent_map_edited.clone();
                let rev_sent_map_edited = rev_sent_map_edited.clone();
                async move {
                    let bot = &upd.bot;
                    let update = &upd.update;
                    let unique_id = MessageMeta::from_message(update);
                    let maybe_me = upd.bot.get_me().send().await;
                    if let Ok(me) = maybe_me {
                        let me = &me.user;
                        let reply =
                            process_ctx(me, &mut *rev_sent_map_edited.lock().unwrap(), update).map(
                                |reply| {
                                    let text = update.text().unwrap();
                                    if text.starts_with(EXPLAIN_COMMAND) {
                                        elaborate(update, reply)
                                    } else {
                                        reply.unwrap_or_else(|err| error_report(err).into())
                                    }
                                },
                            );

                        match reply {
                            None => {
                                let maybe_reply_id =
                                    sent_map_edited.lock().unwrap().get_mut(&unique_id).cloned();
                                if let Some(reply_id) = maybe_reply_id {
                                    log::info!(
                                        "Edited: {} [Withdraw]",
                                        update.text().unwrap_or("<empty>")
                                    );
                                    bot.delete_message(reply_id.chat_id, reply_id.message_id)
                                        .send()
                                        .await
                                        .log_on_error()
                                        .await;
                                    sent_map_edited.lock().unwrap().remove(&unique_id);
                                }
                            }
                            Some(segments) => {
                                let text = segments.text();
                                let entities = segments.entities();

                                let maybe_reply_id =
                                    sent_map_edited.lock().unwrap().get_mut(&unique_id).cloned();
                                let sent_reply = match maybe_reply_id {
                                    None => {
                                        log::info!(
                                            "Edited: {} New: {}",
                                            update.text().unwrap_or("<empty>"),
                                            text
                                        );
                                        upd.reply_to(text).entities(entities).send().await
                                    }
                                    Some(reply_id) => {
                                        log::info!(
                                            "Edited: {} Update: {}",
                                            update.text().unwrap_or("<empty>"),
                                            text
                                        );
                                        bot.edit_message_text(
                                            reply_id.chat_id,
                                            reply_id.message_id,
                                            text.clone(),
                                        )
                                        .entities(entities.clone())
                                        .send()
                                        .await
                                    }
                                };

                                if let Ok(sent_reply) = sent_reply {
                                    let mut sent_map = sent_map_edited.lock().unwrap();
                                    sent_map.insert(
                                        MessageMeta::from_message(update),
                                        MessageMeta::from_message(&sent_reply),
                                    );
                                    let mut rev_sent_map = rev_sent_map_edited.lock().unwrap();
                                    rev_sent_map.insert(
                                        MessageMeta::from_message(&sent_reply),
                                        MessageMeta::from_message(update),
                                    );
                                } else {
                                    log::error!("Failed to reply/edit message.")
                                }
                            }
                        };
                    }
                }
            })
        })
        .dispatch()
        .await;
}
