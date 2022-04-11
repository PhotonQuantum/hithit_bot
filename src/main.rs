#![allow(
    clippy::non_ascii_literal,
    clippy::wildcard_imports,
    clippy::module_name_repetitions,
    clippy::useless_transmute,
    clippy::default_trait_access
)]

use std::fmt::Debug;
use std::sync::Arc;

use futures_core::future::BoxFuture;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use sentry::integrations::tracing::EventFilter;
use sentry::{ClientOptions, IntoDsn};
use teloxide::error_handlers::ErrorHandler;
use teloxide::prelude2::*;
use teloxide::Bot;
use teloxide_listener::Listener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

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
mod utils;

const EXPLAIN_COMMAND: &str = "/explain";
//noinspection RsTypeCheck
//#[allow(clippy::semicolon_if_nothing_returned)]
//const EXPLAIN_COMMAND_EXTENDED: &str = concatcp!(EXPLAIN_COMMAND, "@", BOT_NAME);
static EXPLAIN_COMMAND_EXTENDED: OnceCell<String> = OnceCell::new();
static COMMAND_PREFIX: OnceCell<char> = OnceCell::new();

#[tokio::main]
async fn main() {
    EXPLAIN_COMMAND_EXTENDED
        .set(format!(
            "{}@{}",
            EXPLAIN_COMMAND,
            std::env::var("BOT_NAME").unwrap_or_else(|_| option_env!("BOT_NAME")
                .unwrap_or("hithit_rs_bot")
                .to_string())
        ))
        .unwrap();
    COMMAND_PREFIX
        .set(
            std::env::var("HITHIT_BOT_PREFIX")
                .unwrap_or_else(|_| {
                    option_env!("HITHIT_BOT_PREFIX_BUILD")
                        .unwrap_or("^")
                        .to_string()
                })
                .chars()
                .next()
                .unwrap_or('^'),
        )
        .unwrap();

    let _guard = sentry::init(ClientOptions {
        dsn: std::env::var("SENTRY_DSN")
            .ok()
            .and_then(|dsn| dsn.into_dsn().ok().flatten()),
        release: Some(env!("VERGEN_GIT_SHA").into()),
        traces_sample_rate: 0.5,
        ..Default::default()
    });

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().and_then(tracing_subscriber::fmt::layer()))
        .with(sentry::integrations::tracing::layer().event_filter(|meta| {
            if meta.fields().field("sentry_ignore").is_some() {
                EventFilter::Ignore
            } else {
                sentry::integrations::tracing::default_event_filter(meta)
            }
        }))
        .init();

    log::warn!("Starting hithit bot");

    let bot = Bot::from_env().auto_send();

    let booking = Arc::new(Mutex::new(ReplyBooking::with_capacity(8192)));

    let listener = Listener::from_env_with_prefix("APP_")
        .build(bot.clone())
        .await;

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
    .dispatch_with_listener(listener, Arc::new(TracingErrorHandler))
    .await;
}

struct TracingErrorHandler;

impl<E> ErrorHandler<E> for TracingErrorHandler
where
    E: Debug,
{
    fn handle_error(self: Arc<Self>, error: E) -> BoxFuture<'static, ()> {
        tracing::error!("Error occur from update listener: {:?}", error);

        Box::pin(async {})
    }
}
