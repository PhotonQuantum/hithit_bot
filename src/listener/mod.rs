use teloxide::dispatching::update_listeners;
use teloxide::prelude::*;

#[cfg(feature = "webhook")]
pub mod webhook;

pub enum Listener {
    Polling,
    #[cfg(feature = "webhook")]
    Webhook(webhook::HTTPConfig),
}

impl Listener {
    pub fn from_env() -> Self {
        if let (Some(base), Some(path), Some(addr)) = (
            option_env!("APP_WEBHOOK_URL"),
            option_env!("APP_WEBHOOK_PATH"),
            option_env!("APP_BIND_ADDR"),
        ) {
            #[cfg(not(feature = "webhook"))]
            panic!("webhook support not enabled");
            #[cfg(feature = "webhook")]
            Self::Webhook(webhook::HTTPConfig::new(base, path, addr))
        } else {
            Self::Polling
        }
    }

    #[allow(clippy::future_not_send)]
    pub async fn dispatch_with_me(
        self,
        mut dispatcher: Dispatcher<AutoSend<Bot>>,
        bot: AutoSend<Bot>,
    ) {
        let error_handler =
            LoggingErrorHandler::with_custom_text("An error from the update listener");
        match self {
            Listener::Polling => {
                dispatcher
                    .dispatch_with_listener(
                        update_listeners::polling_default(bot).await,
                        error_handler,
                    )
                    .await;
            }
            #[cfg(feature = "webhook")]
            Listener::Webhook(config) => {
                dispatcher
                    .dispatch_with_listener(webhook::listener(bot, config).await, error_handler)
                    .await;
            }
        }
    }
}
