# Plan: no video in UI — device ws frames arrive fragmented and are dropped

## Goal
Browser UI (vite dev + nginx dist) shows telemetry but never video. Root cause lives in
the relay, not the UI.

## Diagnosis (evidence)
- Firmware sends each camera frame as ONE ws text message: `{"cam":"<b64 jpeg>"}`,
  ~12-22 KB (OV3660 QVGA jpeg <= 15 KB fb, b64 +33%).
- `esp_websocket_client` (espressif 1.x) fragments any send larger than
  `buffer_size` (`esp_cam_tank/src/ws.rs` CLIENT_BUFFER_SIZE = 1024) into continuation
  frames: TEXT(fin=0) + CONT(fin=0)*N + CONT(fin=1). Verified in the vendored C source:
  `esp_websocket_client_send_text` -> `send_with_opcode` -> `send_with_exact_opcode`
  chunks at `client->buffer_size` and manages the FIN bit per chunk.
- actix-ws 0.3.1 `MessageStream` maps continuations to `Message::Continuation` (no
  reassembly). `remote/src/ws.rs::read_from_client` only matches `Message::Text`;
  everything else falls into `Some(Ok(_)) => {}` -> camera frames silently dropped.
- Small messages (`{"dir":..}` 13 B, `{"heap":..}` ~30 B) fit one frame -> controls and
  telemetry work, which masked the bug. `scripts/cam.mjs` passed because Node's
  WebSocket sends one unfragmented frame.
- Serial 2026-08-16: `wifi up` -> `ws connected` -> host `Error: x Broken pipe`
  (espflash lost USB; chip likely reset right after the first big send — separate
  device-side issue to watch after the relay fix).

## Tasks
- [x] 0. Reproduce against running backend: `scripts/cam-frag.mjs` (raw-TCP ws client
      sending 1 KB fragments exactly like the ESP) -> `message timeout` (frame dropped)
- [x] 1. Fix `remote/src/ws.rs`: `ws_stream.aggregate_continuations()` + match
      `AggregatedMessage` (reassembles continuations, default 1 MiB cap)
- [x] 2. `cargo check` + `cargo clippy --all-targets -- -D warnings` in `remote/`: clean
- [x] 3. Rebuilt `remote-backend` container; `cam-frag.mjs` (32 KB / 33 fragments),
      `cam.mjs`, `smoke.mjs` all green
- [ ] 4. On-device verify: reflash not needed (fw unchanged); watch serial past
      `ws connected` — if the chip still resets (Broken pipe), debug brownout/crash
      under sustained WiFi TX (that would block video regardless of this fix)

## Result
- Root cause confirmed and fixed server-side: `read_from_client` only handled
  `Message::Text`, but camera frames arrive as 1 KB continuation fragments
  (esp_websocket_client `buffer_size` = 1024) -> silently dropped by `Some(Ok(_)) => {}`.
  `aggregate_continuations()` reassembles them. UI (vite dev + nginx dist) needed no
  change — frames simply never reached it.
- Live repro before fix: cam-frag timeout; after rebuild: `cam frag relay OK: 32768
  b64 chars in 33 fragments`.
