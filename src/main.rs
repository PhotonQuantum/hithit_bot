#![allow(
    clippy::non_ascii_literal,
    clippy::wildcard_imports,
    clippy::module_name_repetitions,
    clippy::useless_transmute,
    clippy::default_trait_access
)]

use std::env;
use std::fmt::Debug;
use std::sync::Arc;

use futures_core::future::BoxFuture;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use sentry::integrations::tracing::EventFilter;
use sentry::{ClientOptions, IntoDsn};
use teloxide::dispatching::{Dispatcher, UpdateFilterExt};
use teloxide::error_handlers::ErrorHandler;
use teloxide::types::{Message, Update};
use teloxide::update_listeners;
use teloxide::{dptree, Bot};
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

    let url = std::env::var("BOT_SERVER").unwrap_or_else(|_| {
        option_env!("BOT_SERVER")
            .unwrap_or("https://api.telegram.org")
            .to_string()
    });
    let bot = Bot::from_env().set_api_url(url.parse().expect("Parse telegram bot api url error."));

    let booking = Arc::new(Mutex::new(ReplyBooking::with_capacity(8192)));

    let mut dp = Dispatcher::builder(
        bot.clone(),
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
    .enable_ctrlc_handler()
    .build();

    if let (Ok(base), Ok(path), Ok(addr)) = (
        env::var("APP_WEBHOOK_URL"),
        env::var("APP_WEBHOOK_PATH"),
        env::var("APP_BIND_ADDR"),
    ) {
        Box::pin(
            dp.dispatch_with_listener(
                update_listeners::webhooks::axum(
                    bot,
                    update_listeners::webhooks::Options::new(
                        addr.parse().expect("invalid bind address"),
                        base.parse().expect("invalid base url"),
                    )
                    .path(path),
                )
                .await
                .expect("failed to start webhook"),
                Arc::new(TracingErrorHandler),
            ),
        )
        .await;
    } else {
        Box::pin(dp.dispatch_with_listener(
            update_listeners::polling_default(bot).await,
            Arc::new(TracingErrorHandler),
        ))
        .await;
    }
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
