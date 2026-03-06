use std::time::Duration;

use reqwest::multipart;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, warn};

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

    pub async fn send_photo(&self, photo_path: &str) {
        info!("Sending Telegram photo from {}", photo_path);
        let bytes = match tokio::fs::read(photo_path).await {
            Ok(b) => b,
            Err(e) => {
                warn!("Could not read camera snapshot {}: {}", photo_path, e);
                return;
            }
        };

        let url = format!("https://api.telegram.org/bot{}/sendPhoto", self.bot_token);
        let file_part = multipart::Part::bytes(bytes)
            .file_name("snapshot.png")
            .mime_str("image/png")
            .expect("invalid mime type");
        let form = multipart::Form::new()
            .text("chat_id", self.chat_id.clone())
            .part("photo", file_part);

        match self.client.post(&url).multipart(form).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Telegram photo sent from {}", photo_path);
                } else {
                    error!("Failed to send Telegram photo: {}", response.status());
                }
            }
            Err(e) => {
                error!("Telegram photo request error: {}", e);
            }
        }
    }
}
