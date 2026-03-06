use std::sync::Arc;

use rumqttc::{AsyncClient, QoS};
use spawned_concurrency::tasks::{
    send_after, CallResponse, CastResponse, GenServer, GenServerHandle, InitResult,
};
use tokio::time::Duration;
use tracing::info;

use crate::constants::*;
use crate::telegram::TelegramClient;

#[derive(Clone)]
pub enum AlarmCommand {
    Arm,
    Deactivate,
    AutoDeactivate,
    SensorTriggered(SensorType),
}

#[derive(Clone)]
pub enum SensorType {
    FrontDoor,
    BackDoor,
    Movement1,
    Movement2,
    Movement3,
}

#[derive(Clone, Debug)]
pub struct AlarmStatus {
    pub armed: bool,
    pub activated: bool,
}

pub struct AlarmActor {
    armed: bool,
    activated: bool,
    mqtt_client: AsyncClient,
    telegram: Arc<TelegramClient>,
    self_handle: Option<GenServerHandle<Self>>,
}

impl AlarmActor {
    pub fn new(mqtt_client: AsyncClient, telegram: Arc<TelegramClient>) -> Self {
        Self {
            armed: false,
            activated: false,
            mqtt_client,
            telegram,
            self_handle: None,
        }
    }

    async fn publish(&self, topic: &str, payload: &str) {
        if let Err(e) = self
            .mqtt_client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .await
        {
            tracing::error!("Failed to publish to {}: {}", topic, e);
        }
    }

    async fn send_telegram(&self, message: &str) {
        self.telegram.send(message).await;
    }

    async fn arm(&mut self) {
        if !self.armed {
            self.publish(TOPIC_ALARM_INTERIOR, "ON").await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.publish(TOPIC_ALARM_INTERIOR, "OFF").await;
        }
        self.armed = true;
        info!("Alarm armed");
    }

    async fn activate(&mut self, handle: &GenServerHandle<Self>) {
        if !self.armed {
            return;
        }
        info!("Alarm activated");
        self.publish(TOPIC_ALARM_EXTERIOR, "ON").await;

        if !self.activated {
            send_after(
                Duration::from_secs(ALARM_ACTIVE_DURATION),
                handle.clone(),
                AlarmCommand::AutoDeactivate,
            );
        }
        self.activated = true;
    }

    async fn deactivate(&mut self) {
        self.publish(TOPIC_ALARM_EXTERIOR, "OFF").await;
        self.publish(TOPIC_ALARM_INTERIOR, "OFF").await;
        if self.armed && !self.activated {
            self.publish(TOPIC_ALARM_INTERIOR, "ON").await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.publish(TOPIC_ALARM_INTERIOR, "OFF").await;
        }
        self.armed = false;
        self.activated = false;
        info!("Alarm deactivated");
    }

    async fn auto_deactivate(&mut self) {
        if self.activated {
            info!("Alarm auto-deactivated after {} seconds", ALARM_ACTIVE_DURATION);
            self.publish(TOPIC_ALARM_EXTERIOR, "OFF").await;
            self.publish(TOPIC_ALARM_INTERIOR, "OFF").await;
            self.armed = false;
            self.activated = false;
        }
    }

    fn sensor_messages(sensor: &SensorType) -> &'static str {
        match sensor {
            SensorType::FrontDoor => TELEGRAM_MSG_FRONT_DOOR_ALERT,
            SensorType::BackDoor => TELEGRAM_MSG_BACK_DOOR_ALERT,
            SensorType::Movement1 => TELEGRAM_MSG_MOVEMENT_1_ALERT,
            SensorType::Movement2 => TELEGRAM_MSG_MOVEMENT_2_ALERT,
            SensorType::Movement3 => TELEGRAM_MSG_MOVEMENT_3_ALERT,
        }
    }
}

impl GenServer for AlarmActor {
    type CallMsg = ();
    type CastMsg = AlarmCommand;
    type OutMsg = AlarmStatus;
    type Error = Box<dyn std::error::Error + Send>;

    async fn init(
        mut self,
        handle: &GenServerHandle<Self>,
    ) -> Result<InitResult<Self>, Self::Error> {
        self.self_handle = Some(handle.clone());
        info!("AlarmActor started");
        Ok(InitResult::Success(self))
    }

    async fn handle_call(
        &mut self,
        _message: Self::CallMsg,
        _handle: &GenServerHandle<Self>,
    ) -> CallResponse<Self> {
        CallResponse::Reply(AlarmStatus {
            armed: self.armed,
            activated: self.activated,
        })
    }

    async fn handle_cast(
        &mut self,
        message: Self::CastMsg,
        handle: &GenServerHandle<Self>,
    ) -> CastResponse {
        match message {
            AlarmCommand::Arm => {
                self.arm().await;
            }
            AlarmCommand::Deactivate => {
                self.deactivate().await;
            }
            AlarmCommand::AutoDeactivate => {
                self.auto_deactivate().await;
            }
            AlarmCommand::SensorTriggered(sensor) => {
                if self.armed {
                    self.send_telegram(TELEGRAM_MSG_INTRUDER_ALERT).await;
                    self.send_telegram(Self::sensor_messages(&sensor)).await;
                    self.telegram.send_photo(CAM0_SNAPSHOT_PATH).await;
                    self.telegram.send_photo(CAM1_SNAPSHOT_PATH).await;
                    self.activate(handle).await;
                }
            }
        }
        CastResponse::NoReply
    }
}
