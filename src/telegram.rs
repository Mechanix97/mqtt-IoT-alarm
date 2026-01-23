use dotenv::dotenv;
use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::error::Error;
use tracing::info;

lazy_static! {
    static ref BOT_TOKEN: String = {
        dotenv().ok(); // Carga las variables de entorno desde .env
        env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN should be defined .env")
    };
    static ref CHAT_ID: String = {
        dotenv().ok(); // Carga las variables de entorno desde .env
        env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID should be defined .env")
    };
}

pub async fn send_telegram_message(message: &str) -> Result<(), Box<dyn Error>> {
    info!("Sending Telegram message");
    let client = Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", *BOT_TOKEN);
    let payload = json!({
        "chat_id": *CHAT_ID,
        "text": message,
    });

    let response = client.post(&url).json(&payload).send().await?;

    if response.status().is_success() {
        info!("Telegram message sent");
        Ok(())
    } else {
        Err(format!("Error al enviar mensaje: {}", response.status()).into())
    }
}
