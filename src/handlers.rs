use std::sync::Arc;

use eyre::Result;
use parking_lot::Mutex;
use teloxide::prelude2::*;

use crate::elaborator::{elaborate, elaborate_error};
use crate::error::{Error, ErrorExt};
use crate::memory::{MessageMeta, ReplyBooking};
use crate::process::process;
use crate::EXPLAIN_COMMAND;

pub async fn message_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    booking: Arc<Mutex<ReplyBooking>>,
) -> Result<()> {
    let me = &bot.get_me().await?.user;
    let output = process(me, booking.lock(), &msg).lift_should_not_handle()?;

    let reply = if msg.text().unwrap().starts_with(EXPLAIN_COMMAND) {
        elaborate(&msg, output)
    } else {
        output.unwrap_or_else(|e| elaborate_error(e).into())
    };

    let sent_reply = bot
        .send_message(msg.chat.id, reply.text())
        .entities(reply.entities())
        .reply_to_message_id(msg.id)
        .await?;

    booking.lock().book(msg.into(), sent_reply.into());

    Ok(())
}

pub async fn edited_message_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    booking: Arc<Mutex<ReplyBooking>>,
) -> Result<()> {
    let unique_id = MessageMeta::from(&msg);

    let me = &bot.get_me().await?.user;
    let output = process(me, booking.lock(), &msg);

    if let Err(Error::ShouldNotHandle) = output {
        // this is no longer a valid msg, delete previous reply
        let reply_id = booking.lock().forward_lookup(&unique_id).cloned();
        if let Some(reply_id) = reply_id {
            bot.delete_message(reply_id.chat_id, reply_id.message_id)
                .await
                .log_on_error()
                .await;
            booking.lock().forget(&unique_id);
        }
        return Ok(());
    }

    let reply = if msg.text().unwrap().starts_with(EXPLAIN_COMMAND) {
        elaborate(&msg, output)
    } else {
        output.unwrap_or_else(|e| elaborate_error(e).into())
    };

    let reply_id = booking.lock().forward_lookup(&unique_id).cloned();
    let sent_reply = if let Some(reply_id) = reply_id {
        bot.edit_message_text(reply_id.chat_id, reply_id.message_id, reply.text())
            .entities(reply.entities())
            .await
    } else {
        bot.send_message(msg.chat.id, reply.text())
            .entities(reply.entities())
            .reply_to_message_id(msg.id)
            .await
    }?;

    booking.lock().book(msg.into(), sent_reply.into());

    Ok(())
}
