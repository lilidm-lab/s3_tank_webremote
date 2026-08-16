import assert from "node:assert";

const BASE_WS = "ws://127.0.0.1:8080";
const TIMEOUT_MS = 5000;

function dial(path) {
  const ws = new WebSocket(`${BASE_WS}${path}`);
  const opened = new Promise((resolve, reject) => {
    ws.onopen = () => resolve();
    ws.onerror = () => reject(new Error(`ws connect failed: ${path}`));
  });
  return { ws, opened };
}

function nextMessage(ws) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("message timeout")), TIMEOUT_MS);
    ws.addEventListener(
      "message",
      (m) => {
        clearTimeout(timer);
        resolve(JSON.parse(m.data));
      },
      { once: true },
    );
  });
}

const { ws: ui, opened: uiOpen } = dial("/ws/ui");
const hello = nextMessage(ui);
await uiOpen;
assert.deepEqual(await hello, { evt: "device", online: false });

const { ws: dev, opened: devOpen } = dial("/ws/device");
const online = nextMessage(ui);
await devOpen;
assert.deepEqual(await online, { evt: "device", online: true });

ui.send(JSON.stringify({ dir: "up" }));
assert.deepEqual(await nextMessage(dev), { dir: "up" });

dev.send(JSON.stringify({ battery: 4.2 }));
const telemetry = await nextMessage(ui);
assert.equal(telemetry.evt, "telemetry");
assert.equal(telemetry.data.battery, 4.2);

const offline = nextMessage(ui);
dev.close();
assert.deepEqual(await offline, { evt: "device", online: false });

ui.close();
console.log("smoke OK");
process.exit(0);
