# Plan: ESP32-CAM Tank Firmware (camera + DC motors + LEDC + WS to actix)

## Goal
Firmware for the tank device: connects WiFi, registers on the actix remote server
(`remote/`, plan 00) over websocket `/ws/device`, drives 2 DC motors (LEDC PWM +
direction pins) from `{"dir":...}` commands, and streams OV2640 JPEG frames as
telemetry JSON.

Board: ESP32-S3-WROOM **N16R8** (16MB flash, 8MB octal PSRAM) + OV2640
(camera pin map = Freenove ESP32-S3-WROOM CAM; adjust `src/pins.rs` + `src/motor.rs` if wiring differs).

## Tasks
- [x] 0. Retarget to esp32s3: `.cargo/config.toml`, `rust-toolchain.toml` (esp channel), `sdkconfig.defaults` (PSRAM octal/80M, 16MB flash, 64KB cache), `partitions.csv` (3M factory), CI xtensa toolchain
- [x] 1. Deps + camera component: Cargo.toml (`anyhow`, `base64`, extra_components esp32-camera + esp_websocket_client), `bindings.h`
- [x] 2. Firmware modules: `proto.rs` (Direction/Track/parse/encode), `pins.rs`, `wifi.rs`, `motor.rs` (LEDC + GPIO), `camera.rs` (esp32-camera + b64 frames), `ws.rs` (client + tx thread), `main.rs` (wire-up)
- [x] 3. `cargo check` + `cargo clippy --all-targets -- -D warnings` green on xtensa-esp32s3-espidf
- [x] 4. README (wiring, env vars, flash) + close plan

## Notes
- Wire protocol fixed by `remote/src/protocol.rs`: in `{"dir":"up|down|left|right|stop"}`,
  out any JSON -> UI telemetry. Camera frames sent as `{"cam":"<base64 jpeg>"}`,
  status as `{"heap":n,"uptime_s":n}`.
- Camera is the `esp_cam` crate (github.com/lidm0707/ESP32-S3-WROOM-N16R8-OV2640-by-RUST-bindgen-C-)
  initialized via `Camera::new` with QVGA JPEG, `GRAB_LATEST`, framebuffers in PSRAM
  (`src/pins.rs` removed; pin consts live in `src/camera.rs`).
- Threads (message passing, no shared state): ws-callback -> mpsc -> motor thread
  (owns Tank: 2x PinDriver + 2x LedcDriver); camera thread -> mpsc -> ws tx thread
  (owns EspWebSocketClient; it is Send but not Sync). Bounded channels give backpressure.
- LEDC: camera XCLK owns TIMER_0/CH_0 (C side); motors use TIMER_1 + CH_2/CH_3, 10-bit @ 20kHz.
- Build-time env (no secrets in repo): `WIFI_SSID`, `WIFI_PASS`, `WS_URL`
  (e.g. `ws://<server>:8080/ws/device` direct or `:8081` via nginx).
- Benchmark rule not applicable: pure IO relay, no new algorithm (per AGENTS.md).

## Result
- `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`: clean
- `cargo build --release`: links, image ~1.9MB (3M factory partition)
- Component manager resolved `espressif/esp32-camera 2.1.7` (+ `esp_jpeg`) and `esp_websocket_client` against ESP-IDF v5.5.3 — C side builds clean
- Pitfalls learned:
  - esp32-camera headers export an extern static literally named `resolution`; merged into root bindings it collides with esp-idf-hal's `use esp_idf_sys::*` (params named `resolution` in rmt.rs) → must use `bindings_module = "camera"` to isolate camera bindings
  - Changing `[package.metadata.esp-idf-sys]` in Cargo.toml does NOT rerun the esp-idf-sys build script (only tracked files like bindings.h / sdkconfig.defaults do) → `touch bindings.h` after metadata edits
  - esp-idf-hal master PinDriver API: `PinDriver<'d, MODE>` with constructors (`PinDriver::output(pin)`), no pin type param; `Peripherals` needs full destructure before moving fields around
  - esp32-camera 2.1.x: `pin_sccb_sda/scl` live in anonymous unions (`__bindgen_anon_1/2`), enums renamed `camera_fb_location_t` / `camera_grab_mode_t`, `xclk_freq_hz` and `sccb_i2c_port` are `c_int`, `fb_count` is `usize`
- Runtime TODO on hardware: verify large (>4KB) ws TX frames send OK (b64 QVGA jpegs ~15-30KB per frame); if the C client rejects them, chunk or shrink framesize/quality

## Hardware bring-up fixes (2026-08-16)
- Symptom: boot-crash loop `E i2c: CONFLICT! driver_ng is not allowed to be used with this old driver` -> abort.
  IDF v5.5 ships a **link-time** guard (constructor in legacy `i2c.c`): if `i2c_master.c`
  (driver_ng) is linked at all, legacy-driver users (esp32-camera SCCB) abort at boot.
  Fix: `CONFIG_I2C_SKIP_LEGACY_CONFLICT_CHECK=y` in `sdkconfig.defaults`. Current dep
  graph no longer links driver_ng, flag kept as insurance.
- Swapped hand-rolled `src/camera.rs` bindings for the `esp_cam` crate (user's lib, same
  Freenove pin map). `default_ov2640()` (fb in DRAM, fb_count 1) spam-logged `cam_hal:
  FB-OVF` on esp32-camera 2.1.7: DRAM fb is sized smaller than real JPEG frames
  (`!psram_mode` branch in `cam_hal.c`), so every frame overflowed. Fix: `Camera::new`
  with `CAMERA_FB_IN_PSRAM` + `GRAB_LATEST` + QVGA — verified on hardware: camera init
  OK, fb_get succeeds, zero FB-OVF over 30 s.
- Camera thread now spawns before Wi-Fi connect so camera bring-up is independent of
  build-time env (WIFI_SSID/WIFI_PASS/WS_URL). Dummy-env build verified on device:
  Wi-Fi times out gracefully (`ESP_ERR_TIMEOUT`, app_main returns, no reboot loop);
  real-creds rebuild still required before actual use.
- Board sensor is actually an **OV3660** (boot log `Camera PID=0x3660`), not OV2640.
  Symptom: `cam_hal: NO-SOI - JPEG start marker missing` on ~every frame (throttle
  prints 1st + every 100th; ~6 fps of garbage, no valid JPEG) at the crate-default
  20MHz XCLK. esp32-camera is tuned for OV3660 JPEG at 16MHz XCLK (`ov3660.c` has a
  `xclk_freq_hz == 16000000` PLL branch; git history "Enable EDMA for JPEG when XCLK
  is 16MHz"). Fix in `src/camera.rs`: runtime PID detect (`SensorModel` enum via
  `esp_camera_sensor_get().id.PID` vs `camera_pid_t_*_PID` consts) + `tune_ov3660()`:
  `set_xclk(16MHz)` -> reapply `set_framesize(QVGA)` (recomputes sensor PLL) ->
  `vflip(1)`/`brightness(+1)`/`saturation(-2)` (Arduino OV3660 defaults). OV2640 path
  unchanged. Verified on hardware 2026-08-16: boot log shows the second PLL line
  `VCO: 128MHz / SYSCLK: 32MHz / PCLK: 8MHz` (16MHz branch active) and zero NO-SOI.
  Remaining `ESP_ERR_TIMEOUT` = dummy-creds Wi-Fi timeout; real-creds rebuild still
  required before actual use.
- Symptom: one motor runs by default while the tank is unconnected. Cause: `main.rs`
  ran `Tank::new` AFTER `wifi::connect`, and on Wi-Fi timeout/error `app_main`
  returned before motor init ever ran -> GPIO1/2 (left IN1/IN2) float, GPIO39-42 sit
  in JTAG pull-up state -> undefined H-bridge input. Fix: `Tank::new` + `motor::spawn`
  moved BEFORE `wifi::connect` (pins low + duty 0 within ~2s of boot; safe on Wi-Fi
  failure). ws.rs audited: `FrameType::Text(false)` maps to `esp_websocket_client_send_text`
  (complete frame, C client auto-fragments >buffer_size payloads with FIN on last
  chunk) -> 1KB client buffer is fine for ~32KB b64 cam frames.
- No-video triage verified server-side on the running containers (2026-08-16):
  `remote/scripts/cam.mjs` (new) relays a 32KB `{"cam":...}` device->UI, `smoke.mjs`
  green, deployed `remote-web-1` serves the cam-capable dist (index-CSJSBaZq.js).
  Remaining unknown is the device hop: needs real-creds rebuild + `WS_URL` = Mac LAN
  IP (not localhost), then serial should show `wifi up, ip: ...` -> `ws connected`
  -> `tank online`. macOS firewall must allow inbound 8080/8081 on the LAN.
- Resolution (2026-08-16, plan 03): device hop reached (`wifi up` + `ws connected` in
  serial) but frames died in the relay — esp_websocket_client fragments >1KB sends
  into continuations, which the old `read_from_client` dropped. Fixed with
  `aggregate_continuations()` + rebuilt backend. Open: serial session ended with
  host `Error: x Broken pipe` right after `ws connected` (chip reset under first
  sustained WiFi TX?) — verify on hardware.
- FreeRTOS task hierarchy (2026-08-16): all Rust threads were spawning at the
  pthread default (prio 5, no core affinity). Added `src/tasks.rs` with
  `Priority` enum (WsClient=14 > Motor=12 > WsTx=8 > Camera=6; app band 2-17
  keeps clear of lwIP=18, esp_timer=22, WiFi=23 in this IDF v5.5.3 build) +
  `tasks::configure()` (esp-idf-hal `ThreadSpawnConfiguration` ->
  `esp_pthread_set_cfg`, applied right before each `Builder::spawn`). User call:
  the **ws client task is first tier** — it is the RX entry for motor commands
  and the disconnect->Stop safety path, so it must never lag under camera
  load; CLIENT_TASK_PRIO now derives from `Priority::WsClient` (was hardcoded
  5). svc `EspWebSocketClientConfig` exposes `task_prio` but NOT
  `task_core_id`, so the client task stays tskNO_AFFINITY (effectively lands
  on Core 0 next to the net stack since motor+cam own Core 1). Note: motor
  (12, pinned Core 1) wakes the moment the client task (14) blocks after the
  callback — us-level delay, accepted. `Builder::stack_size` still wins over
  cfg stack (verified in IDF pthread.c: attr->stacksize overrides cfg).
  Thread names `motor`/`cam`/`ws_tx` now show in task dumps. cargo check +
  clippy clean; runtime priority behavior pending hardware verification.
