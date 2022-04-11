use parking_lot::MutexGuard;
use teloxide::prelude2::*;
use teloxide::types::User;

use crate::error::{Error, Result};
use crate::formatter::FormatContext;
use crate::memory::ReplyBooking;
use crate::parser::Parser;
use crate::segments::{Segment, Segments};
use crate::{COMMAND_PREFIX, EXPLAIN_COMMAND, EXPLAIN_COMMAND_EXTENDED};

pub fn process(
    bot_user: &User,
    booking: MutexGuard<ReplyBooking>,
    msg: &Message,
) -> Result<Segments> {
    let text = msg.text().ok_or(Error::ShouldNotHandle)?;
    let entities = msg.entities().ok_or(Error::ShouldNotHandle)?;

    if !text.starts_with('/') {
        return Err(Error::ShouldNotHandle);
    }

    let fmt_ctx = build_format_ctx(bot_user, booking, msg)?;

    let parser = if text.starts_with(EXPLAIN_COMMAND_EXTENDED.get().unwrap()) {
        Segments::build(text, entities)
            .drain_head(EXPLAIN_COMMAND_EXTENDED.get().unwrap().len() + 1)
            .map(|segments| Parser::new(segments, true))
    } else if text.starts_with(EXPLAIN_COMMAND) {
        Segments::build(text, entities)
            .drain_head(EXPLAIN_COMMAND.len() + 1)
            .map(|segments| Parser::new(segments, true))
    } else {
        text.chars().nth(1).and_then(|chr| {
            if chr.len_utf8() > 1 {
                Segments::build(text, entities)
                    .drain_head(1)
                    .map(|segments| Parser::new(segments, true))
            } else if chr == *COMMAND_PREFIX.get().unwrap() {
                Segments::build(text, entities)
                    .drain_head(2)
                    .map(|segments| Parser::new(segments, true))
            } else {
                Segments::build(text, entities)
                    .drain_head(1)
                    .map(|segments| Parser::new(segments, false))
            }
        })
    }
    .ok_or(Error::ShouldNotHandle)?;

    let formatter = parser.try_as_formatter()?;

    Ok(formatter.format(&fmt_ctx)?)
}

fn get_reply_user(
    bot_user: &User,
    mut booking: MutexGuard<ReplyBooking>,
    message: &Message,
) -> Option<Segment> {
    Some(if let Some(reply_msg) = message.reply_to_message() {
        let user = reply_msg.from()?;
        if user == bot_user {
            let cached_msg = booking.reverse_lookup(&reply_msg.try_into().ok()?);
            cached_msg.map_or_else(
                || user.into(),
                |msg| {
                    let sender = &msg.sender;
                    message.from().map_or_else(
                        || sender.into(),
                        |curr_sender| {
                            if sender == curr_sender {
                                Segment::from_user_with_name(user.clone(), String::from("自己"))
                            } else {
                                sender.into()
                            }
                        },
                    )
                },
            )
        } else {
            user.into()
        }
    } else {
        Segment::from_user_with_name(message.from()?.clone(), String::from("自己"))
    })
}

fn build_format_ctx(
    bot_user: &User,
    booking: MutexGuard<ReplyBooking>,
    msg: &Message,
) -> Result<FormatContext> {
    let sender = Segment::from_user(msg.from().ok_or(Error::ShouldNotHandle)?.clone());
    let me = Segment::from_user_with_name(
        msg.from().ok_or(Error::ShouldNotHandle)?.clone(),
        String::from("自己"),
    );
    let receiver = get_reply_user(bot_user, booking, msg).ok_or(Error::ShouldNotHandle)?;
    Ok(FormatContext::new(sender, receiver, me))
}
