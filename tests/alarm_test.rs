use std::sync::Arc;

use mqtt_broker_alarm::alarm::*;
use mqtt_broker_alarm::parse_on_off;
use mqtt_broker_alarm::telegram::TelegramClient;
use rumqttc::{AsyncClient, MqttOptions};
use spawned_concurrency::tasks::GenServer;
use tokio::time::{sleep, Duration};

fn setup() -> (AsyncClient, Arc<TelegramClient>) {
    let mqttoptions = MqttOptions::new("test-client", "127.0.0.1", 1883);
    let (client, _eventloop) = AsyncClient::new(mqttoptions, 10);
    let telegram = Arc::new(TelegramClient::new(
        "dummy_token".to_string(),
        "dummy_chat".to_string(),
    ));
    (client, telegram)
}

async fn get_status(handle: &mut spawned_concurrency::tasks::GenServerHandle<AlarmActor>) -> AlarmStatus {
    handle.call(()).await.expect("Failed to get alarm status")
}

#[tokio::test]
async fn test_initial_state() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(!status.armed, "Should start disarmed");
    assert!(!status.activated, "Should start deactivated");
}

#[tokio::test]
async fn test_arm() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle.cast(AlarmCommand::Arm).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    let status = get_status(&mut handle).await;
    assert!(status.armed, "Should be armed after Arm command");
    assert!(!status.activated, "Should not be activated");
}

#[tokio::test]
async fn test_deactivate_after_arm() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle.cast(AlarmCommand::Arm).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    handle.cast(AlarmCommand::Deactivate).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(!status.armed, "Should be disarmed after Deactivate");
    assert!(!status.activated, "Should not be activated");
}

#[tokio::test]
async fn test_sensor_when_disarmed() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle
        .cast(AlarmCommand::SensorTriggered(SensorType::FrontDoor))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(!status.armed, "Should remain disarmed");
    assert!(!status.activated, "Should not activate when disarmed");
}

#[tokio::test]
async fn test_sensor_when_armed() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle.cast(AlarmCommand::Arm).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    handle
        .cast(AlarmCommand::SensorTriggered(SensorType::FrontDoor))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(status.armed, "Should remain armed");
    assert!(status.activated, "Should be activated after sensor trigger");
}

#[tokio::test]
async fn test_auto_deactivate() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle.cast(AlarmCommand::Arm).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    handle
        .cast(AlarmCommand::SensorTriggered(SensorType::Movement1))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    // Simulate what send_after would do
    handle.cast(AlarmCommand::AutoDeactivate).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(!status.armed, "Should be disarmed after auto-deactivate");
    assert!(!status.activated, "Should be deactivated after auto-deactivate");
}

#[tokio::test]
async fn test_multiple_sensors_single_activation() {
    let (client, telegram) = setup();
    let mut handle = AlarmActor::new(client, telegram).start();
    sleep(Duration::from_millis(50)).await;

    handle.cast(AlarmCommand::Arm).await.unwrap();
    sleep(Duration::from_millis(200)).await;

    // First sensor triggers activation
    handle
        .cast(AlarmCommand::SensorTriggered(SensorType::FrontDoor))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    // Second sensor while already activated
    handle
        .cast(AlarmCommand::SensorTriggered(SensorType::BackDoor))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(status.armed);
    assert!(status.activated);

    // Auto-deactivate still resets everything
    handle.cast(AlarmCommand::AutoDeactivate).await.unwrap();
    sleep(Duration::from_millis(50)).await;

    let status = get_status(&mut handle).await;
    assert!(!status.armed);
    assert!(!status.activated);
}

#[test]
fn test_parse_on_off() {
    assert!(parse_on_off(b"ON"));
    assert!(!parse_on_off(b"OFF"));
    assert!(!parse_on_off(b""));
    assert!(!parse_on_off(b"on"));
    assert!(!parse_on_off(b"garbage"));
}
