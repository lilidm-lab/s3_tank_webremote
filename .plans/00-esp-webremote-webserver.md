# Plan: ESP Web Remote Server (Actix + Vite + Nginx + Docker)

## Goal
Actix-web websocket server where the ESP32 registers as device; browser UI (arrow pad) sends direction commands back to the ESP. Served through Nginx, wired with docker compose.

## Tasks
- [x] 0. Scaffold backend crate deps + gitignore entries
- [x] 1. Backend: direction protocol enums, registry actor, device/ui ws sessions, main (cargo check + clippy green)
- [x] 2. Frontend: yarn + vite + TS, arrow pad UI, ws client with reconnect (yarn build green)
- [x] 3. Infra: nginx conf, backend Dockerfile, web Dockerfile, docker-compose, .dockerignore, README
- [x] 4. Validation: e2e ws relay smoke test (node), docker compose config, close plan

## Notes
- Wire protocol (JSON):
  - UI -> server: `{"dir":"up|down|left|right|stop"}` (same frame forwarded to device)
  - device -> server: any JSON object -> broadcast to UIs as `{"evt":"telemetry","data":{...}}`
  - server -> UI: `{"evt":"device","online":true|false}` on device connect/disconnect
- Endpoints: `GET /ws/device` (ESP registers), `GET /ws/ui` (browser), `GET /health`
- No Mutex/RwLock needed: registry state is owned by a single actix Actor (single writer), per style "no over engineer"
- Ports: backend 8080 (direct ESP/dev), web via nginx host 8081:80
- Benchmark rule not applicable: IO relay, no new algorithm (noted per AGENTS.md)

## Result
- cargo check / clippy --all-targets / fmt --check: clean
- yarn build (tsc strict + vite 7.3.6): clean, dist emitted
- `node scripts/smoke.mjs` against running backend: relay verified (hello/offline events, dir forward, telemetry forward)
- `docker compose config`: valid
- Pitfall learned: node undici WebSocket dispatches `open` and already-buffered first frame in the same tick -> attach message listeners BEFORE awaiting open (fixed in scripts/smoke.mjs)
- ESP32 side: connect ws://<host>:8081/ws/device (via nginx) or ws://<host>:8080/ws/device direct; send {"dir":...} frames arrive from server
