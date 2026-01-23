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
- **deactivate:** Turns off both alarms and disarms the system

The auto-deactivation spawned task publishes OFF to exterior, interior, and alarm status topics independently.

### telegram.rs - Notifications

Uses `lazy_static` to load `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` from environment variables (via dotenv). Creates a new HTTP client per message and POSTs to the Telegram sendMessage API.

### constants.rs - Configuration

Defines all MQTT topic strings, the broker address/port, alert messages (in Spanish with emojis), and the alarm active duration (60 seconds).

## Known Bugs

1. **Alarm `activated` state never reset by auto-deactivation** (`alarm.rs:59-93`): The spawned task sends OFF messages after 60 seconds but cannot mutate `self.activated` back to `false`. After auto-deactivation, the struct still has `activated = true`, which means:
   - The disarm beep won't play (guarded by `!self.activated` in `disarm()`)
   - Subsequent `activate()` calls won't spawn new deactivation timers (guarded by `!self.activated`)
   - Note: The spawned task publishes OFF to `alarm/status`, which triggers `deactivate()` in the event loop indirectly. But this creates duplicate OFF publishes and a convoluted state flow.

2. **No MQTT reconnection strategy** (`main.rs:49-108`): When the MQTT connection drops, the inner `while let` exits and the outer `loop` immediately retries with no backoff delay, potentially causing a tight reconnection loop that spins the CPU.

3. **Retained MQTT messages can cause false alarms on restart** (`alarm.yaml`): Door and motion sensors publish with `retain: true`. When the Rust bridge reconnects or restarts, it receives the last retained state of all sensors. If a door was left open, the bridge immediately triggers the alarm upon reconnection (if armed).

4. **Auto-deactivation sends duplicate OFF publishes** (`alarm.rs:62-93`): The spawned task publishes OFF to exterior/interior topics, then publishes OFF to `alarm/status`. The event loop receives the status OFF and calls `deactivate()`, which publishes OFF to exterior/interior again. This results in redundant MQTT messages.

5. **`parse_on_off` silently ignores invalid payloads** (`main.rs:112-118`): Unknown or corrupted sensor payloads are treated as OFF with no warning log. This can mask hardware or communication issues that should be investigated.

6. **Telegram error message in Spanish** (`telegram.rs:35`): The error string `"Error al enviar mensaje"` is in Spanish while all log messages use English. Minor language inconsistency.

7. **Bell events processed regardless of alarm state**: The doorbell sends a Telegram notification even when the alarm is disarmed. This may or may not be intentional depending on the desired behavior.

8. **WiFi credentials in plaintext** (`alarm.yaml:8-9`): The ESPHome config contains WiFi SSID and password fields in plaintext. Although currently placeholders, the pattern encourages putting real credentials in a version-controlled file.

## Possible Improvements

1. **Reuse HTTP client**: Create a single `reqwest::Client` instance (e.g., in a `lazy_static` or passed as parameter) instead of constructing one per Telegram message. This enables connection pooling and reduces overhead.

2. **Fix auto-deactivation state sync**: Use `Arc<Mutex<bool>>` or a `tokio::sync::watch` channel to share the `activated` flag with the spawned deactivation task, or restructure to use `tokio::select!` with a sleep future in the main loop instead of spawning a detached task.

3. **Add reconnection backoff**: Implement exponential backoff when the MQTT event loop disconnects to avoid a CPU-spinning tight loop.

4. **Make network config configurable**: Move `MQTT_SERVER_IP` and `MQTT_SERVER_PORT` to environment variables instead of compile-time constants, enabling deployment flexibility without recompilation.

5. **Use QoS AtLeastOnce for sensor subscriptions**: Currently using `AtMostOnce` (QoS 0) for all subscriptions. Security-critical sensors like doors and motion should use `AtLeastOnce` (QoS 1) to avoid missed alerts.

6. **Extract repeated alarm trigger pattern**: The door/motion handlers repeat the same logic (send intruder alert + specific alert + activate). This could be extracted into a helper function to reduce duplication.

7. **Add entry/exit delay**: A configurable delay before activating the alarm after arming (exit delay) and before triggering after a sensor event (entry delay) would reduce false alarms.

8. **Add Telegram send retry logic**: If the Telegram API returns an error or the network is down, the current code propagates the error and could crash the event loop. A retry with backoff would improve reliability.

9. **Replace `lazy_static` with `std::sync::OnceLock`**: Since Rust 1.70+, `OnceLock` is in std and removes the need for the `lazy_static` crate.

10. **Add unit tests**: The alarm state machine logic is testable in isolation with a mock MQTT client.

11. **Add alarm status feedback via Telegram**: Send a confirmation message when the alarm is armed/disarmed so users get feedback that their command was received.

12. **Handle retained messages on startup**: Ignore or filter retained messages during initial subscription to prevent false alarm triggers from stale sensor states.

13. **Use ESPHome `secrets.yaml`**: Move WiFi credentials to a `secrets.yaml` file (ESPHome convention) excluded from version control instead of hardcoding in alarm.yaml.

14. **Log unknown MQTT payloads**: Add a warning log in `parse_on_off` for unrecognized payloads to help diagnose sensor communication issues.
