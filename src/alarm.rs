use tokio::time::{Duration, sleep};

use rumqttc::AsyncClient;
use std::error::Error;
use tokio::task;
use tracing::info;

use crate::constants::{
    ALARM_ACTIVE_DURATION, TOPIC_ALARM_EXTERIOR, TOPIC_ALARM_INTERIOR, TOPIC_ALARM_STATUS,
};

pub struct Alarm<'a> {
    armed: bool,
    activated: bool,
    client: &'a AsyncClient,
}

impl<'a> Alarm<'a> {
    pub fn new(client: &'a AsyncClient) -> Self {
        Self {
            armed: false,
            activated: false,
            client,
        }
    }

    pub async fn arm(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_armed() {
            self.publish(TOPIC_ALARM_INTERIOR, "ON").await?;
            sleep(Duration::from_millis(100)).await;
            self.publish(TOPIC_ALARM_INTERIOR, "OFF").await?;
        }
        info!("alarm armed");
        self.armed = true;
        Ok(())
    }

    pub async fn disarm(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_armed() && !self.activated {
            self.publish(TOPIC_ALARM_INTERIOR, "ON").await?;
            sleep(Duration::from_millis(100)).await;
            self.publish(TOPIC_ALARM_INTERIOR, "OFF").await?;
        }
        info!("alarm disarmed");
        self.armed = false;
        Ok(())
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub async fn activate(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_armed() {
            info!("Alarm activated");
            self.publish(TOPIC_ALARM_EXTERIOR, "ON").await?;
            // self.publish(TOPIC_ALARM_INTERIOR, "ON").await?;

            if !self.activated {
                // Spawn a task to deactivate the alarm after 1 minute
                let client = self.client.clone();
                task::spawn(async move {
                    sleep(Duration::from_secs(ALARM_ACTIVE_DURATION)).await;
                    info!("Alarm automatically deactivated after 1 minute");
                    if let Err(e) = client
                        .publish(
                            TOPIC_ALARM_EXTERIOR,
                            rumqttc::QoS::AtLeastOnce,
                            false,
                            "OFF",
                        )
                        .await
                    {
                        tracing::error!("Failed to deactivate alarm: {}", e);
                    }
                    if let Err(e) = client
                        .publish(
                            TOPIC_ALARM_INTERIOR,
                            rumqttc::QoS::AtLeastOnce,
                            false,
                            "OFF",
                        )
                        .await
                    {
                        tracing::error!("Failed to deactivate alarm: {}", e);
                    }
                    if let Err(e) = client
                        .publish(TOPIC_ALARM_STATUS, rumqttc::QoS::AtLeastOnce, false, "OFF")
                        .await
                    {
                        tracing::error!("Failed to deactivate alarm: {}", e);
                    }
                });
            }
            self.activated = true;
        }
        Ok(())
    }

    pub async fn deactivate(&mut self) -> Result<(), Box<dyn Error>> {
        self.publish(TOPIC_ALARM_EXTERIOR, "OFF").await?;
        self.publish(TOPIC_ALARM_INTERIOR, "OFF").await?;
        self.disarm().await?;
        self.activated = false;
        Ok(())
    }

    pub async fn publish(&self, topic: &str, payload: &str) -> Result<(), Box<dyn Error>> {
        self.client
            .publish(topic, rumqttc::QoS::AtLeastOnce, false, payload)
            .await?;
        Ok(())
    }
}
