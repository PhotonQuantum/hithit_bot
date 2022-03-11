#![allow(
    clippy::non_ascii_literal,
    clippy::wildcard_imports,
    clippy::module_name_repetitions
)]

use std::sync::Arc;

use const_format::concatcp;
use parking_lot::Mutex;
use teloxide::prelude2::*;
use teloxide::Bot;
use teloxide_listener::Listener;
use tracing_subscriber::EnvFilter;

use crate::handlers::{edited_message_handler, message_handler};
use crate::memory::ReplyBooking;

mod elaborator;
mod error;
mod formatter;
mod handlers;
mod memory;
mod parser;
mod process;
mod segments;

const EXPLAIN_COMMAND: &str = "/explain";
const BOT_NAME: &str = "hithit_rs_bot";
//noinspection RsTypeCheck
#[allow(clippy::useless_transmute, clippy::semicolon_if_nothing_returned)]
const EXPLAIN_COMMAND_EXTENDED: &str = concatcp!(EXPLAIN_COMMAND, "@", BOT_NAME);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    log::warn!("Starting hithit bot");

    let bot = Bot::from_env().auto_send();

    let booking = Arc::new(Mutex::new(ReplyBooking::with_capacity(8192)));

    let listener = Listener::from_env_with_prefix("APP_")
        .build(bot.clone())
        .await;
    let error_handler = LoggingErrorHandler::with_custom_text("An error from the update listener");

    Dispatcher::builder(
        bot,
        dptree::entry()
            .branch(Update::filter_message().branch(
                dptree::filter(|msg: Message| msg.text().is_some()).endpoint(message_handler),
            ))
            .branch(
                Update::filter_edited_message().branch(
                    dptree::filter(|msg: Message| msg.text().is_some())
                        .endpoint(edited_message_handler),
                ),
            ),
    )
    .dependencies(dptree::deps![booking])
    .build()
    .setup_ctrlc_handler()
    .dispatch_with_listener(listener, error_handler)
    .await;
}
