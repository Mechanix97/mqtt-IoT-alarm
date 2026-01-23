# MQTT Broker Alarm

## Project Overview

An IoT home security alarm system built with Rust and ESP32 that monitors multiple sensors (motion, door, doorbell) via MQTT and sends real-time Telegram notifications when triggered.

**Architecture:** ESP32 sensors -> MQTT Broker -> Rust Bridge -> Telegram Bot API

## Tech Stack

- **Language:** Rust (Edition 2024)
- **Async Runtime:** Tokio 1.38
- **Actor Framework:** spawned-concurrency 0.4 (Erlang-style GenServer)
- **MQTT Client:** rumqttc 0.24
- **HTTP Client:** reqwest 0.12
- **IoT Firmware:** ESPHome on ESP32
- **Deployment:** Docker (multi-stage build with cargo-chef)
- **Notifications:** Telegram Bot API

## Project Structure

```
src/
  main.rs        - MQTT event loop, topic routing, actor startup
  alarm.rs       - AlarmActor GenServer (state machine + MQTT publish)
  telegram.rs    - TelegramActor GenServer (HTTP client + Bot API)
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

## Architecture

Uses an actor model (Erlang-style GenServer via `spawned-concurrency`):

```
MQTT EventLoop ──cast──> AlarmActor ──cast──> TelegramActor
                              │                     │
                              │ (publish)           │ (HTTP POST)
                              ▼                     ▼
                         MQTT Broker          Telegram API
```

## Code Analysis

### main.rs - Event Router

Loads environment variables, starts both actors, connects to MQTT broker at `192.168.100.2:1883`, subscribes to 7 sensor topics. The event loop is a pure router with no mutable state - it pattern-matches incoming MQTT events and casts commands to actors. The `is_armed()` check is handled inside the AlarmActor.

### alarm.rs - AlarmActor (GenServer)

Manages alarm state (`armed`, `activated`) and an MQTT client for publishing. Receives `AlarmCommand` messages via cast:

- **Arm:** Publishes ON/OFF pulse to interior alarm (beep), sets `armed = true`
- **SensorTriggered(sensor):** If armed, sends intruder alert + sensor-specific alert via TelegramActor, activates exterior alarm
- **AutoDeactivate:** Scheduled via `send_after(60s)` - resets `activated` and `armed` to false, publishes OFF
- **Deactivate:** Manual deactivation - turns off both alarms, resets state

The auto-deactivation uses `send_after` to deliver `AutoDeactivate` back to itself after 60 seconds, ensuring state is properly reset within the actor's message handler (no race conditions).

### telegram.rs - TelegramActor (GenServer)

Holds a persistent `reqwest::Client` (connection pooling). Receives message strings via cast and POSTs to the Telegram Bot API. Errors are logged but don't crash the event loop (fire-and-forget pattern).

### constants.rs - Configuration

Defines all MQTT topic strings, the broker address/port, alert messages (in Spanish with emojis), and the alarm active duration (60 seconds).

## Known Bugs

1. **No MQTT reconnection strategy** (`main.rs:56-93`): When the MQTT connection drops, the inner `while let` exits and the outer `loop` immediately retries with no backoff delay, potentially causing a tight reconnection loop that spins the CPU.

2. **Retained MQTT messages can cause false alarms on restart** (`alarm.yaml`): Door and motion sensors publish with `retain: true`. When the Rust bridge reconnects or restarts, it receives the last retained state of all sensors. If a door was left open, the bridge immediately triggers the alarm upon reconnection (if armed).

3. **`parse_on_off` silently ignores invalid payloads** (`main.rs:96-101`): Unknown or corrupted sensor payloads are treated as OFF with no warning log. This can mask hardware or communication issues that should be investigated.

4. **Bell events processed regardless of alarm state**: The doorbell sends a Telegram notification even when the alarm is disarmed. This may or may not be intentional depending on the desired behavior.

5. **WiFi credentials in plaintext** (`alarm.yaml:8-9`): The ESPHome config contains WiFi SSID and password fields in plaintext. Although currently placeholders, the pattern encourages putting real credentials in a version-controlled file.

## Possible Improvements

1. **Add reconnection backoff**: Implement exponential backoff when the MQTT event loop disconnects to avoid a CPU-spinning tight loop.

2. **Make network config configurable**: Move `MQTT_SERVER_IP` and `MQTT_SERVER_PORT` to environment variables instead of compile-time constants, enabling deployment flexibility without recompilation.

3. **Use QoS AtLeastOnce for sensor subscriptions**: Currently using `AtMostOnce` (QoS 0) for all subscriptions. Security-critical sensors like doors and motion should use `AtLeastOnce` (QoS 1) to avoid missed alerts.

4. **Add entry/exit delay**: A configurable delay before activating the alarm after arming (exit delay) and before triggering after a sensor event (entry delay) would reduce false alarms.

5. **Add Telegram send retry logic**: If the Telegram API returns an error, the TelegramActor logs the error and continues. Adding retry with backoff would improve reliability for transient network issues.

6. **Add unit tests**: The AlarmActor state machine logic is testable in isolation with a mock MQTT client and TelegramActor handle.

7. **Add alarm status feedback via Telegram**: Send a confirmation message when the alarm is armed/disarmed so users get feedback that their command was received.

8. **Handle retained messages on startup**: Ignore or filter retained messages during initial subscription to prevent false alarm triggers from stale sensor states.

9. **Use ESPHome `secrets.yaml`**: Move WiFi credentials to a `secrets.yaml` file (ESPHome convention) excluded from version control instead of hardcoding in alarm.yaml.

10. **Log unknown MQTT payloads**: Add a warning log in `parse_on_off` for unrecognized payloads to help diagnose sensor communication issues.
