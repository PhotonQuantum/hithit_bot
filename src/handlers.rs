use std::sync::Arc;

use eyre::{Result, WrapErr};
use parking_lot::Mutex;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::requests::Requester;
use teloxide::types::{Message, ReplyParameters};
use teloxide::Bot;
use tracing::instrument;

use crate::elaborator::{elaborate, elaborate_error};
use crate::error::{Error, ErrorExt};
use crate::memory::{MessageMeta, ReplyBooking};
use crate::process::process;
use crate::utils::sentry_capture;
use crate::EXPLAIN_COMMAND;

#[instrument(fields(from = %msg.chat.id, msg = ? msg.text()), skip(msg, bot, pool))]
pub async fn compatibility_handler(
    msg: Message,
    bot: Bot,
    mode: bool,
    pool: sqlx::PgPool,
) -> Result<()> {
    if !msg.chat.is_group() && !msg.chat.is_supergroup() {
        bot.send_message(
            msg.chat.id,
            "Compatibility mode is only available in groups and supergroups.",
        )
        .await?;
        return Ok(());
    }

    // Check if the user has the necessary permissions
    let Some(user) = msg.from else {
        bot.send_message(
            msg.chat.id,
            "You must be a member of the group to set compatibility mode.",
        )
        .await?;
        return Ok(());
    };
    let chat_member = bot.get_chat_member(msg.chat.id, user.id).await?;
    if !chat_member.is_privileged() {
        bot.send_message(
            msg.chat.id,
            "You must be an admin to set compatibility mode.",
        )
        .await?;
        return Ok(());
    }

    // Set the compatibility mode
    let result = if mode {
        // Insert id into the database
        sqlx::query!(
            "INSERT INTO compatibility (id) VALUES ($1) ON CONFLICT DO NOTHING",
            msg.chat.id.0
        )
        .execute(&pool)
        .await?
    } else {
        // Remove id from the database
        sqlx::query!("DELETE FROM compatibility WHERE id = $1", msg.chat.id.0)
            .execute(&pool)
            .await?
    };
    let report = match (result.rows_affected() > 0, mode) {
        (true, true) => "Compatibility mode enabled.",
        (false, true) => "Compatibility mode already enabled.",
        (true, false) => "Compatibility mode disabled.",
        (false, false) => "Compatibility mode already disabled.",
    };
    bot.send_message(msg.chat.id, report).await?;
    Ok(())
}

#[instrument(fields(from = %msg.chat.id, msg = ? msg.text()), skip(msg, bot, booking))]
pub async fn message_handler(
    msg: Message,
    bot: Bot,
    booking: Arc<Mutex<ReplyBooking>>,
    pool: sqlx::PgPool,
) -> Result<()> {
    let me = &sentry_capture(bot.get_me().await)?.user;

    let output = process(me, &booking, &msg, pool)
        .await
        .lift_should_not_handle()?;

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
            .reply_parameters(ReplyParameters::new(msg.id))
            .await
            .wrap_err("Cannot send reply message"),
    )?;

    booking.lock().book(
        sentry_capture(msg.try_into())?,
        sentry_capture(sent_reply.try_into())?,
    );

    Ok(())
}

pub async fn edited_message_handler(
    msg: Message,
    bot: Bot,
    booking: Arc<Mutex<ReplyBooking>>,
    pool: sqlx::PgPool,
) -> Result<()> {
    let unique_id = sentry_capture(MessageMeta::try_from(&msg))?;

    let me = sentry_capture(bot.get_me().await)?.user;
    let output = process(&me, &booking, &msg, pool).await;

    if matches!(output, Err(Error::ShouldNotHandle)) {
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
                .reply_parameters(ReplyParameters::new(msg.id))
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
