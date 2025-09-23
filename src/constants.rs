pub const TOPIC_ALARM_STATUS: &str = "alarm/status";

pub const TOPIC_FRONT_DOOR: &str = "alarm/front_door";
pub const TOPIC_BACK_DOOR: &str = "alarm/back_door";

pub const TOPIC_MOVEMENT_SENSOR_1: &str = "alarm/movement_1";
pub const TOPIC_MOVEMENT_SENSOR_2: &str = "alarm/movement_2";
pub const TOPIC_MOVEMENT_SENSOR_3: &str = "alarm/movement_3";

pub const TOPIC_BELL: &str = "bell";

pub const TOPIC_ALARM_EXTERIOR: &str = "alarm/exterior";
pub const TOPIC_ALARM_INTERIOR: &str = "alarm/interior";

pub const MQTT_SERVER_IP: &str = "192.168.100.2";
pub const MQTT_SERVER_PORT: u16 = 1883;

pub const TELEGRAM_MSG_BELL_ALERT: &str = "Timbreee 👏👏";
pub const TELEGRAM_MSG_INTRUDER_ALERT: &str = "🚨 ALERTA INTRUSO 🚨";
pub const TELEGRAM_MSG_FRONT_DOOR_ALERT: &str = "Puerta delantera abierta 🚪";
pub const TELEGRAM_MSG_BACK_DOOR_ALERT: &str = "Puerta trasera abierta 🚪";
pub const TELEGRAM_MSG_MOVEMENT_1_ALERT: &str = "Movimiento detectado 🏃🏾‍♂️ Sector 1";
pub const TELEGRAM_MSG_MOVEMENT_3_ALERT: &str = "Movimiento detectado 🏃🏾‍♂️ Sector 2";
pub const TELEGRAM_MSG_MOVEMENT_2_ALERT: &str = "Movimiento detectado 🏃🏾‍♂️ Sector 3";

pub const ALARM_ACTIVE_DURATION: u64 = 60;
