use std::net::SocketAddr;
use std::str::FromStr;

use axum::handler::get;
use axum::{Json, Router};
use serde_json::Value;
use teloxide::dispatching::stop_token::AsyncStopToken;
use teloxide::dispatching::update_listeners::{StatefulListener, UpdateListener};
use teloxide::prelude::*;
use teloxide::types::Update;
use tokio::sync::mpsc::unbounded_channel;
use tokio_stream::wrappers::UnboundedReceiverStream;
use url::Url;

pub struct HTTPConfig {
    pub base_url: Url,
    pub path: String,
    pub addr: SocketAddr,
}

impl HTTPConfig {
    pub fn new(base_url: &str, path: &str, addr: &str) -> Self {
        Self {
            base_url: Url::parse(base_url).expect("invalid base url"),
            path: path.to_string(),
            addr: SocketAddr::from_str(addr).expect("invalid bind addr"),
        }
    }
}

impl HTTPConfig {
    fn full_url(&self) -> Url {
        self.base_url.join(self.path.as_str()).expect("invalid url")
    }
}

struct State<S, T> {
    stream: S,
    stop_tx: T,
}

impl<S, T> State<S, T> {
    fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl<S, T: Clone> State<S, T> {
    fn stop_tx(&mut self) -> T {
        self.stop_tx.clone()
    }
}

pub async fn listener(
    bot: AutoSend<Bot>,
    config: HTTPConfig,
) -> impl UpdateListener<serde_json::Error> {
    bot.set_webhook(config.full_url())
        .await
        .expect("unable to setup webhook");

    let (tx, rx) = unbounded_channel();

    let app = Router::new().route(
        config.path.as_str(),
        get(move |Json(payload): Json<Value>| async move {
            tx.send(Update::try_parse(&payload))
                .expect("unable to send update to dispatcher");
        }),
    );

    let (stop_tx, stop_rx) = AsyncStopToken::new_pair();

    let srv = axum::Server::bind(&config.addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(stop_rx);

    tokio::spawn(srv);

    let stream = UnboundedReceiverStream::new(rx);

    StatefulListener::new(State { stream, stop_tx }, State::stream_mut, State::stop_tx)
}
