use mqtt_broker_alarm::alarm::*;
use mqtt_broker_alarm::constants::*;
use mqtt_broker_alarm::parse_on_off;
use mqtt_broker_alarm::telegram::*;

use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenv::dotenv;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use spawned_concurrency::tasks::GenServer;
use tokio::time::Duration;
use tracing::{error, info};

async fn subscribe_all(client: &AsyncClient) -> Result<(), rumqttc::ClientError> {
    client
        .subscribe(TOPIC_ALARM_STATUS, QoS::AtMostOnce)
        .await?;
    client.subscribe(TOPIC_BELL, QoS::AtMostOnce).await?;
    client.subscribe(TOPIC_FRONT_DOOR, QoS::AtMostOnce).await?;
    client.subscribe(TOPIC_BACK_DOOR, QoS::AtMostOnce).await?;
    client
        .subscribe(TOPIC_MOVEMENT_SENSOR_1, QoS::AtMostOnce)
        .await?;
    client
        .subscribe(TOPIC_MOVEMENT_SENSOR_2, QoS::AtMostOnce)
        .await?;
    client
        .subscribe(TOPIC_MOVEMENT_SENSOR_3, QoS::AtMostOnce)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenv().ok();
    let bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let chat_id = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID must be set");

    info!("Connecting to MQTT broker");

    let mut mqttoptions = MqttOptions::new("rust-mqtt-alarm", MQTT_SERVER_IP, MQTT_SERVER_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(30));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);

    info!("Starting actors");

    let telegram = Arc::new(TelegramClient::new(bot_token, chat_id));
    let mut alarm_handle = AlarmActor::new(client.clone(), telegram.clone()).start();

    info!("Starting event loop");
    let mut backoff = Duration::from_secs(1);
    loop {
        match eventloop.poll().await {
            Ok(event) => {
                backoff = Duration::from_secs(1);
                match event {
                    Event::Incoming(Packet::ConnAck(_)) => {
                        info!("Connected to MQTT broker, subscribing to topics");
                        if let Err(e) = subscribe_all(&client).await {
                            error!("Failed to subscribe: {}", e);
                        }
                    }
                    Event::Incoming(Packet::Publish(p)) => match p.topic.as_str() {
                        TOPIC_BELL => {
                            info!("Bell event");
                            let tg = telegram.clone();
                            tokio::spawn(async move {
                                tg.send(TELEGRAM_MSG_BELL_ALERT).await;
                                tg.send_photo(CAM0_SNAPSHOT_PATH).await;
                            });
                        }
                        TOPIC_ALARM_STATUS => match parse_on_off(&p.payload) {
                            true => alarm_handle.cast(AlarmCommand::Arm).await?,
                            false => alarm_handle.cast(AlarmCommand::Deactivate).await?,
                        },
                        TOPIC_FRONT_DOOR if parse_on_off(&p.payload) => {
                            info!("Front door opened");
                            alarm_handle
                                .cast(AlarmCommand::SensorTriggered(SensorType::FrontDoor))
                                .await?;
                        }
                        TOPIC_BACK_DOOR if parse_on_off(&p.payload) => {
                            info!("Back door opened");
                            alarm_handle
                                .cast(AlarmCommand::SensorTriggered(SensorType::BackDoor))
                                .await?;
                        }
                        TOPIC_MOVEMENT_SENSOR_1 if parse_on_off(&p.payload) => {
                            info!("Movement sector 1");
                            alarm_handle
                                .cast(AlarmCommand::SensorTriggered(SensorType::Movement1))
                                .await?;
                        }
                        TOPIC_MOVEMENT_SENSOR_2 if parse_on_off(&p.payload) => {
                            info!("Movement sector 2");
                            alarm_handle
                                .cast(AlarmCommand::SensorTriggered(SensorType::Movement2))
                                .await?;
                        }
                        TOPIC_MOVEMENT_SENSOR_3 if parse_on_off(&p.payload) => {
                            info!("Movement sector 3");
                            alarm_handle
                                .cast(AlarmCommand::SensorTriggered(SensorType::Movement3))
                                .await?;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            Err(e) => {
                error!(
                    "MQTT connection error: {}. Reconnecting in {:?}",
                    e, backoff
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        }
    }
}
