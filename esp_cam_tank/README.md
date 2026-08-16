# esp_cam_tank

ESP32-S3 tank firmware: OV2640 camera stream + 2 DC motors (LEDC PWM) driven over
websocket from the actix remote server (`../remote`).

Board: ESP32-S3-WROOM-N16R8 (16MB flash, 8MB octal PSRAM). Camera comes from the
`esp_cam` crate (github.com/lidm0707/...-OV2640-by-RUST-bindgen-C-); the Freenove
ESP32-S3-WROOM CAM pin map is stolen in `src/camera.rs` (QVGA JPEG, PSRAM
framebuffers). Motor pins live in `src/motor.rs`.

## Wiring (L298N / TB6612-style driver)

| signal | GPIO |
|---|---|
| left IN1 / IN2 / PWM | 1 / 2 / 42 |
| right IN1 / IN2 / PWM | 41 / 40 / 39 |

LEDC: camera XCLK uses TIMER_0/CH_0; motors use TIMER_1 + CH_2/CH_3 (10-bit @ 20 kHz).

## Protocol

Connects to `WS_URL` (`/ws/device` endpoint). Receives `{"dir":"up|down|left|right|stop"}`,
sends telemetry: `{"cam":"<base64 jpeg>"}` (~10 fps QVGA) and `{"heap":n,"uptime_s":n}` (1 s).
On disconnect the motors stop.

## Build / flash

Build-time env (no secrets in repo):

```sh
cp .env.sample .env   # fill in values
./build.sh            # source .env + cargo build --release
```

`./build.sh` = `set -a; source .env; set +a; cargo build --release` with a guard
that all three vars are set (override the path via `ENV_FILE=...`).

Flash + monitor:

```sh
cargo run --release
```

Xtensa build uses the `esp` rustup toolchain (espup). Toolchain / flash size /
PSRAM settings: `rust-toolchain.toml`, `.cargo/config.toml`, `sdkconfig.defaults`.
