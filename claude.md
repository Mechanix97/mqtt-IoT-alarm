# MQTT Broker Alarm

## Project Overview

An IoT home security alarm system built with Rust and ESP32 that monitors multiple sensors (motion, door, doorbell) via MQTT and sends real-time Telegram notifications when triggered.

**Architecture:** ESP32 sensors -> MQTT Broker -> Rust Bridge -> Telegram Bot API

## Tech Stack

- **Language:** Rust (Edition 2024)
- **Async Runtime:** Tokio 1.38
- **MQTT Client:** rumqttc 0.24
- **HTTP Client:** reqwest 0.12
- **IoT Firmware:** ESPHome on ESP32
- **Deployment:** Docker (multi-stage build with cargo-chef)
- **Notifications:** Telegram Bot API

## Project Structure

```
src/
  main.rs        - MQTT event loop, topic routing, payload parsing
  alarm.rs       - Alarm state machine (arm/disarm/activate/deactivate)
  telegram.rs    - Telegram Bot API integration
  constants.rs   - MQTT topics, messages, and configuration constants
esphome/
  alarm.yaml     - ESP32 sensor/actuator configuration
Dockerfile       - Multi-stage Docker build
Makefile         - Build and run shortcuts
```

## How to Run

```bash
# Development (requires secrets/telegram.env with TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID)
make run

# Docker
make run-docker

# Stop
make stop-docker
```

## Code Analysis

### main.rs - Event Loop

The main module sets up an MQTT connection to `192.168.100.2:1883` and subscribes to 7 sensor topics. It runs an infinite polling loop that pattern-matches incoming MQTT publish events. Sensor events (doors, motion) are only processed when the alarm is armed. The `parse_on_off` function converts `"ON"`/`"OFF"` byte payloads to booleans.

The event handling for doors and motion sensors follows a repeated pattern: check if payload is ON, send intruder alert + specific alert via Telegram, then activate the alarm.

### alarm.rs - State Machine

The `Alarm` struct holds two boolean states (`armed`, `activated`) and a reference to the MQTT client. Key behaviors:

- **arm/disarm:** Sends a quick ON/OFF pulse to the interior alarm (audible feedback beep)
- **activate:** Turns on the exterior alarm and spawns an async task that auto-deactivates after 60 seconds
- **desactivate:** Turns off both alarms and disarms the system

The auto-deactivation spawned task publishes OFF to exterior, interior, and alarm status topics independently.

### telegram.rs - Notifications

Uses `lazy_static` to load `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` from environment variables (via dotenv). Creates a new HTTP client per message and POSTs to the Telegram sendMessage API.

### constants.rs - Configuration

Defines all MQTT topic strings, the broker address/port, alert messages (in Spanish with emojis), and the alarm active duration (60 seconds).

## Known Bugs

1. **Swapped movement sensor messages** (`constants.rs:23-24`): `TELEGRAM_MSG_MOVEMENT_2_ALERT` contains the text "Sector 3" and `TELEGRAM_MSG_MOVEMENT_3_ALERT` contains "Sector 2". The sector numbers in the message strings are swapped relative to their constant names.

2. **Typo in log messages** (`main.rs:80,88`): Logs say "Momvement" instead of "Movement".

3. **Alarm `activated` state never reset by auto-deactivation** (`alarm.rs:59-93`): The spawned task sends OFF messages after 60 seconds but cannot mutate `self.activated` back to `false`. After auto-deactivation, the struct still has `activated = true`, which means:
   - The disarm beep won't play (guarded by `!self.activated` in `disarm()`)
   - Subsequent `activate()` calls won't spawn new deactivation timers (guarded by `!self.activated`)

4. **Sensitive token logged at INFO level** (`telegram.rs:22`): The `BOT_TOKEN` is printed in logs on every Telegram message send, leaking credentials in log output.

5. **No MQTT reconnection strategy** (`main.rs:49-108`): When the MQTT connection drops, the inner `while let` exits and the outer `loop` immediately retries with no backoff delay, potentially causing a tight reconnection loop.

6. **Spelling: `desactivate`** (`alarm.rs:100`, `main.rs:60`): Should be `deactivate`. Minor naming inconsistency.

7. **Bell events processed regardless of alarm state**: The doorbell sends a Telegram notification even when the alarm is disarmed. This may or may not be intentional depending on the desired behavior.

## Possible Improvements

1. **Reuse HTTP client**: Create a single `reqwest::Client` instance (e.g., in a `lazy_static` or passed as parameter) instead of constructing one per Telegram message. This enables connection pooling and reduces overhead.

2. **Fix auto-deactivation state sync**: Use `Arc<Mutex<bool>>` or a `tokio::sync::watch` channel to share the `activated` flag with the spawned deactivation task, or restructure to use `tokio::select!` with a sleep future in the main loop.

3. **Add reconnection backoff**: Implement exponential backoff when the MQTT event loop disconnects to avoid a CPU-spinning tight loop.

4. **Remove sensitive logging**: Remove or redact the `BOT_TOKEN` and `CHAT_ID` info logs, or gate them behind a DEBUG level.

5. **Make network config configurable**: Move `MQTT_SERVER_IP` and `MQTT_SERVER_PORT` to environment variables instead of compile-time constants, enabling deployment flexibility without recompilation.

6. **Use QoS AtLeastOnce for sensor subscriptions**: Currently using `AtMostOnce` (QoS 0) for all subscriptions. Security-critical sensors like doors and motion should use `AtLeastOnce` (QoS 1) to avoid missed alerts.

7. **Extract repeated alarm trigger pattern**: The door/motion handlers repeat the same logic (send intruder alert + specific alert + activate). This could be extracted into a helper function.

8. **Add entry/exit delay**: A configurable delay before activating the alarm after arming (exit delay) and before triggering after a sensor event (entry delay) would reduce false alarms.

9. **Add Telegram send retry logic**: If the Telegram API returns an error or the network is down, the current code propagates the error and could crash the event loop. A retry with backoff would improve reliability.

10. **Replace `lazy_static` with `std::sync::OnceLock`**: Since Rust 1.70+, `OnceLock` is in std and removes the need for the `lazy_static` crate.

11. **Add unit tests**: The alarm state machine logic is testable in isolation with a mock MQTT client.

12. **Add alarm status feedback via Telegram**: Send a confirmation message when the alarm is armed/disarmed so users get feedback that their command was received.
