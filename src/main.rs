#![allow(
    clippy::non_ascii_literal,
    clippy::wildcard_imports,
    clippy::module_name_repetitions,
    clippy::useless_transmute,
    clippy::default_trait_access
)]

use std::env;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

use futures_core::future::BoxFuture;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use sentry::integrations::tracing::EventFilter;
use sentry::{ClientOptions, IntoDsn};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use teloxide::dispatching::{Dispatcher, UpdateFilterExt};
use teloxide::dptree::case;
use teloxide::error_handlers::ErrorHandler;
use teloxide::macros::BotCommands;
use teloxide::requests::Requester;
use teloxide::types::{Message, Update};
use teloxide::update_listeners;
use teloxide::utils::command::BotCommands as _;
use teloxide::{dptree, Bot};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::handlers::{compatibility_handler, edited_message_handler, message_handler};
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

static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    COMMAND_PREFIX
        .set(
            env::var("HITHIT_BOT_PREFIX")
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
        dsn: env::var("SENTRY_DSN")
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

    let url = env::var("BOT_SERVER").unwrap_or_else(|_| {
        option_env!("BOT_SERVER")
            .unwrap_or("https://api.telegram.org")
            .to_string()
    });
    let bot = Bot::from_env().set_api_url(url.parse().expect("Parse telegram bot api url error."));

    let bot_info = bot.get_me().await.expect("Unable to get bot info.");
    let bot_name = bot_info.username();
    EXPLAIN_COMMAND_EXTENDED
        .set(format!("{EXPLAIN_COMMAND}@{bot_name}"))
        .unwrap();

    let booking = Arc::new(Mutex::new(ReplyBooking::with_capacity(8192)));

    let pg_opts =
        PgConnectOptions::from_str(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .expect("DATABASE_URL must be a valid PG connection string");
    let pg_opts = if let Ok(password) = env::var("DATABASE_PASSWORD") {
        pg_opts.password(&password)
    } else {
        pg_opts
    };
    let pgpool = PgPoolOptions::new()
        .connect_with(pg_opts)
        .await
        .expect("Failed to connect to database");
    MIGRATOR
        .run(&pgpool)
        .await
        .expect("Failed to run migrations");

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![Command::Help].endpoint(|msg: Message, bot: Bot| async move {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "{}\n/explain <message> — elaborate the given message and result.",
                        Command::descriptions()
                    ),
                )
                .await?;
                Ok(())
            }),
        )
        .branch(case![Command::Compatibility(mode)].endpoint(compatibility_handler));
    let mut dp = Dispatcher::builder(
        bot.clone(),
        dptree::entry()
            .branch(Update::filter_message().branch(command_handler).branch(
                dptree::filter(|msg: Message| msg.text().is_some()).endpoint(message_handler),
            ))
            .branch(
                Update::filter_edited_message().branch(
                    dptree::filter(|msg: Message| msg.text().is_some())
                        .endpoint(edited_message_handler),
                ),
            ),
    )
    .dependencies(dptree::deps![booking, pgpool])
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

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "snake_case",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "show this help message.")]
    Help,
    #[command(description = "set compatibility mode. <true/false>")]
    Compatibility(bool),
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
