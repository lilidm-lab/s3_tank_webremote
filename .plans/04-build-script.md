# Plan: firmware build script (.env include + cargo release)

## Goal
One command for the firmware release build: source build-time config from `.env`
(WIFI_SSID / WIFI_PASS / WS_URL baked in via `option_env!`) and run
`cargo build --release`. Replaces the error-prone manual
`set -a; source .env; set +a` dance from the READMEs.

## Tasks
- [x] 0. `esp_cam_tank/build.sh` (POSIX sh): cd to script dir, source `$ENV_FILE`
      (default `.env`, overridable) with `set -a`, guard that all three vars are
      non-empty (no values printed — only WS_URL, which is not a secret), then
      `cargo build --release "$@"` (args pass through)
- [x] 1. Negative paths: missing file -> error + hint, empty var -> error
- [x] 2. Happy path: `./build.sh` -> release build green (3.6 s incremental),
      WS_URL confirmed as the Mac LAN IP (192.168.1.123:8080, per plan 01 note)
- [x] 3. Docs: build sections in `README.md` + `esp_cam_tank/README.md`

## Notes
- `.env` is only sourced by the script at runtime; its contents were never read
  into the session (AGENTS.md rule)
- Sourcing (not parsing) keeps quoting semantics identical to the old manual flow

## Result
- `esp_cam_tank/build.sh` executable, both failure modes and the real build verified
