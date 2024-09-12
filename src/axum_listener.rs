use std::convert::Infallible;
use teloxide::prelude::Requester;
use teloxide::update_listeners::webhooks::{axum_to_router, Options};
use teloxide::update_listeners::UpdateListener;

// Adopted from teloxide::update_listeners::webhooks::axum
pub async fn axum<R>(
    bot: R,
    options: Options,
) -> Result<impl UpdateListener<Err = Infallible>, R::Err>
where
    R: Requester + Send + 'static,
    <R as Requester>::DeleteWebhook: Send,
{
    let Options { address, .. } = options;

    let (mut update_listener, stop_flag, app) = axum_to_router(bot, options).await?;
    // ADD HEALTH CHECK
    let app = app.route("/health-check", axum::routing::get(|| async { "OK" }));
    let stop_token = update_listener.stop_token();

    tokio::spawn(async move {
        let tcp_listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|err| {
                stop_token.stop();
                err
            })
            .expect("Couldn't bind to the address");
        axum::serve(tcp_listener, app)
            .with_graceful_shutdown(stop_flag)
            .await
            .map_err(|err| {
                stop_token.stop();
                err
            })
            .expect("Axum server error");
    });

    Ok(update_listener)
}
