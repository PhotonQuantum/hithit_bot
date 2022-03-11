use std::sync::Arc;

use eyre::{Result, WrapErr};
use parking_lot::Mutex;
use teloxide::prelude2::*;
use tracing::instrument;

use crate::elaborator::{elaborate, elaborate_error};
use crate::error::{Error, ErrorExt};
use crate::memory::{MessageMeta, ReplyBooking};
use crate::process::process;
use crate::utils::sentry_capture;
use crate::EXPLAIN_COMMAND;

#[instrument(fields(from = msg.chat.id, msg = ? msg.text()), skip(msg, bot, booking))]
pub async fn message_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    booking: Arc<Mutex<ReplyBooking>>,
) -> Result<()> {
    let me = &sentry_capture(bot.get_me().await)?.user;
    let output = process(me, booking.lock(), &msg).lift_should_not_handle()?;

    let reply = if msg
        .text()
        .expect("must be text message")
        .starts_with(EXPLAIN_COMMAND)
    {
        elaborate(&msg, output)
    } else {
        output.unwrap_or_else(|e| elaborate_error(e).into())
    };

    let sent_reply = sentry_capture(
        bot.send_message(msg.chat.id, reply.text())
            .entities(reply.entities())
            .reply_to_message_id(msg.id)
            .await
            .wrap_err("Cannot send reply message"),
    )?;

    booking.lock().book(
        sentry_capture(msg.try_into())?,
        sentry_capture(sent_reply.try_into())?,
    );

    Ok(())
}

#[instrument(fields(from = msg.chat.id, msg = ? msg.text()), skip(msg, bot, booking))]
pub async fn edited_message_handler(
    msg: Message,
    bot: AutoSend<Bot>,
    booking: Arc<Mutex<ReplyBooking>>,
) -> Result<()> {
    let unique_id = sentry_capture(MessageMeta::try_from(&msg))?;

    let me = &sentry_capture(bot.get_me().await)?.user;
    let output = process(me, booking.lock(), &msg);

    if let Err(Error::ShouldNotHandle) = output {
        // this is no longer a valid msg, delete previous reply
        let reply_id = booking.lock().forward_lookup(&unique_id).cloned();
        if let Some(reply_id) = reply_id {
            sentry_capture(
                bot.delete_message(reply_id.chat_id, reply_id.message_id)
                    .await
                    .wrap_err("Cannot delete sent message"),
            )?;
            booking.lock().forget(&unique_id);
        }
        return Ok(());
    }

    let reply = if msg
        .text()
        .expect("must be text message")
        .starts_with(EXPLAIN_COMMAND)
    {
        elaborate(&msg, output)
    } else {
        output.unwrap_or_else(|e| elaborate_error(e).into())
    };

    let reply_id = booking.lock().forward_lookup(&unique_id).cloned();
    let sent_reply = if let Some(reply_id) = reply_id {
        sentry_capture(
            bot.edit_message_text(reply_id.chat_id, reply_id.message_id, reply.text())
                .entities(reply.entities())
                .await
                .wrap_err("Cannot edit sent message"),
        )?
    } else {
        sentry_capture(
            bot.send_message(msg.chat.id, reply.text())
                .entities(reply.entities())
                .reply_to_message_id(msg.id)
                .await
                .wrap_err("Cannot reply to edited message"),
        )?
    };

    booking.lock().book(
        sentry_capture(msg.try_into())?,
        sentry_capture(sent_reply.try_into())?,
    );

    Ok(())
}
