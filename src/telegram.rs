use std::time::Duration;

use reqwest::Client;
use serde_json::json;
use tracing::{error, info};

pub struct TelegramClient {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramClient {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            bot_token,
            chat_id,
        }
    }

    pub async fn send(&self, message: &str) {
        info!("Sending Telegram message");
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let payload = json!({
            "chat_id": self.chat_id,
            "text": message,
        });

        match self.client.post(&url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Telegram message sent");
                } else {
                    error!("Failed to send Telegram message: {}", response.status());
                }
            }
            Err(e) => {
                error!("Telegram request error: {}", e);
            }
        }
    }
}
