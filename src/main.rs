#![allow(clippy::non_ascii_literal)]

use std::sync::Arc;

use const_format::concatcp;
use parking_lot::Mutex;
use teloxide::prelude::*;

use crate::listener::Listener;
use crate::memory::ReplyBooking;

#[macro_use]
mod utils;

mod dispatcher;
mod elaborator;
mod error;
mod formatter;
mod listener;
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
    teloxide::enable_logging!();
    log::warn!("Starting hithit bot");

    let bot = Bot::from_env().auto_send();

    let booking = Arc::new(Mutex::new(ReplyBooking::with_capacity(8192)));

    let listener = Listener::from_env();
    let dispatcher = dispatcher::dispatcher(bot.clone(), booking);

    listener.dispatch_with_me(dispatcher, bot).await;
}
