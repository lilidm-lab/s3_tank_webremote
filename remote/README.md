# Tank Web Remote

Web remote for an ESP32 tank. The ESP32 registers as a device over websocket; browsers get an arrow pad that sends direction commands back to the tank.

## Layout

- `src/` — actix-web websocket server (Rust)
- `vite/` — browser UI (yarn + vite + TypeScript)
- `nginx/` — serves the built UI, proxies `/ws/` to the backend
- `Dockerfile`, `Dockerfile.web`, `docker-compose.yml` — deployment

## Protocol (JSON over websocket)

| endpoint | side | messages |
|---|---|---|
| `GET /ws/device` | ESP32 | in: `{"dir":"up\|down\|left\|right\|stop"}`, out: any JSON object (telemetry) |
| `GET /ws/ui` | browser | out: `{"dir":"up\|down\|left\|right\|stop"}`, in: `{"evt":"device","online":bool}` / `{"evt":"telemetry","data":{...}}` |
| `GET /health` | any | `ok` |

## Run

```sh
docker compose up --build
```

- UI: http://localhost:8081
- ESP32 websocket: `ws://<host>:8081/ws/device` (via nginx) or `ws://<host>:8080/ws/device` (direct)

## Dev

```sh
# backend
cargo run

# frontend (proxies /ws to localhost:8080)
cd vite && yarn && yarn dev
```

Controls: arrow buttons (touch/mouse) or keyboard arrows, space = stop. Direction is sent on press, `stop` on release.

## Smoke test

With the backend running (`cargo run`):

```sh
node scripts/smoke.mjs
```
