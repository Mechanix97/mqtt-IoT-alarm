mod alarm;
mod constants;
mod telegram;

use alarm::*;
use constants::*;
use telegram::*;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::error::Error;
use tokio::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Broker started");

    let mut mqttoptions = MqttOptions::new("rust-mqtt-reader", MQTT_SERVER_IP, MQTT_SERVER_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // subscribe to topics
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

    let mut alarm = Alarm::new(&client);

    info!("Starting event loop");
    loop {
        while let Ok(event) = eventloop.poll().await {
            match event {
                Event::Incoming(Packet::Publish(p)) => match p.topic.as_str() {
                    TOPIC_BELL => {
                        info!("topic/bell event");
                        send_telegram_message(TELEGRAM_MSG_BELL_ALERT).await?;
                    }
                    TOPIC_ALARM_STATUS => match parse_on_off(&p.payload) {
                        true => alarm.arm().await?,
                        false => {
                            alarm.deactivate().await?;
                        }
                    },
                    TOPIC_FRONT_DOOR if alarm.is_armed() => {
                        info!("Front door {:?}", p.payload);
                        if parse_on_off(&p.payload) {
                            send_telegram_message(TELEGRAM_MSG_INTRUDER_ALERT).await?;
                            send_telegram_message(TELEGRAM_MSG_FRONT_DOOR_ALERT).await?;
                            alarm.activate().await?;
                        }
                    }
                    TOPIC_BACK_DOOR if alarm.is_armed() => {
                        info!("Back door {:?}", p.payload);
                        if parse_on_off(&p.payload) {
                            send_telegram_message(TELEGRAM_MSG_INTRUDER_ALERT).await?;
                            send_telegram_message(TELEGRAM_MSG_BACK_DOOR_ALERT).await?;
                            alarm.activate().await?;
                        }
                    }
                    TOPIC_MOVEMENT_SENSOR_1 if alarm.is_armed() => {
                        info!("Movement sector 1 {:?}", p.payload);
                        if parse_on_off(&p.payload) {
                            send_telegram_message(TELEGRAM_MSG_INTRUDER_ALERT).await?;
                            send_telegram_message(TELEGRAM_MSG_MOVEMENT_1_ALERT).await?;
                            alarm.activate().await?;
                        }
                    }
                    TOPIC_MOVEMENT_SENSOR_2 if alarm.is_armed() => {
                        info!("Movement sector 2 {:?}", p.payload);
                        if parse_on_off(&p.payload) {
                            send_telegram_message(TELEGRAM_MSG_INTRUDER_ALERT).await?;
                            send_telegram_message(TELEGRAM_MSG_MOVEMENT_2_ALERT).await?;
                            alarm.activate().await?;
                        }
                    }
                    TOPIC_MOVEMENT_SENSOR_3 if alarm.is_armed() => {
                        info!("Movement sector 3 {:?}", p.payload);
                        if parse_on_off(&p.payload) {
                            send_telegram_message(TELEGRAM_MSG_INTRUDER_ALERT).await?;
                            send_telegram_message(TELEGRAM_MSG_MOVEMENT_3_ALERT).await?;
                            alarm.activate().await?;
                        }
                    }
                    _ => {}
                },
                Event::Outgoing(_) => {}
                _ => {}
            }
        }
    }
}

fn parse_on_off(input: &[u8]) -> bool {
    match input {
        b"ON" => true,
        b"OFF" => false,
        _ => false,
    }
}
