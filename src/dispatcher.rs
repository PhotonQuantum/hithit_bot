use std::sync::Arc;

use parking_lot::Mutex;
use teloxide::prelude::*;
use teloxide::types::User;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::elaborator::{elaborate, elaborate_error};
use crate::error::Error;
use crate::memory::{MessageMeta, ReplyBooking};
use crate::process::process;
use crate::EXPLAIN_COMMAND;

pub fn dispatcher(
    bot: AutoSend<Bot>,
    booking: Arc<Mutex<ReplyBooking>>,
) -> Dispatcher<AutoSend<Bot>> {
    Dispatcher::new(bot)
        .messages_handler({
            let booking = booking.clone();
            move |rx: DispatcherHandlerRx<AutoSend<Bot>, Message>| {
                UnboundedReceiverStream::new(rx).for_each_concurrent(None, move |upd| {
                    let booking = booking.clone();
                    async move {
                        let update = &upd.update;
                        let me: &User = &bail!(unwrap upd.requester.get_me().await, Err(_)).user;
                        let output = bail!(
                            process(me, booking.lock(), update),
                            Err(Error::ShouldNotHandle)
                        );

                        let reply = if update.text().unwrap().starts_with(EXPLAIN_COMMAND) {
                            elaborate(update, output)
                        } else {
                            output.unwrap_or_else(|e| elaborate_error(e).into())
                        };

                        let sent_reply = upd.answer(reply.text()).entities(reply.entities()).await;

                        if let Ok(sent_reply) = sent_reply {
                            booking.lock().book(update.into(), sent_reply.into());
                        };
                    }
                })
            }
        })
        .edited_messages_handler({
            move |rx: DispatcherHandlerRx<AutoSend<Bot>, Message>| {
                UnboundedReceiverStream::new(rx).for_each_concurrent(None, move |upd| {
                    let booking = booking.clone();
                    async move {
                        let bot = &upd.requester;
                        let update = &upd.update;
                        let unique_id = MessageMeta::from(update);

                        let me: &User = &bail!(unwrap upd.requester.get_me().await, Err(_)).user;
                        let output = process(me, booking.lock(), update);

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
                            return;
                        }

                        let reply = if update.text().unwrap().starts_with(EXPLAIN_COMMAND) {
                            elaborate(update, output)
                        } else {
                            output.unwrap_or_else(|e| elaborate_error(e).into())
                        };

                        let reply_id = booking.lock().forward_lookup(&unique_id).cloned();
                        let sent_reply = if let Some(reply_id) = reply_id {
                            bot.edit_message_text(
                                reply_id.chat_id,
                                reply_id.message_id,
                                reply.text(),
                            )
                            .entities(reply.entities())
                            .await
                        } else {
                            upd.reply_to(reply.text()).entities(reply.entities()).await
                        };

                        if let Ok(sent_reply) = sent_reply {
                            booking.lock().book(update.into(), sent_reply.into());
                        };
                    }
                })
            }
        })
}
