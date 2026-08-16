import assert from "node:assert";

const BASE_WS = "ws://localhost:8080";
const TIMEOUT_MS = 5000;
const CAM_B64_BYTES = 24 * 1024; // ~QVGA jpeg -> ~32KB base64

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

// minimal valid JPEG (SOI + EOI), padded to a realistic frame size
const jpeg = new Uint8Array(CAM_B64_BYTES);
jpeg[0] = 0xff;
jpeg[1] = 0xd8;
jpeg[2] = 0xff;
jpeg[3] = 0xd9;
const cam = Buffer.from(jpeg).toString("base64");

const { ws: ui, opened: uiOpen } = dial("/ws/ui");
await uiOpen;

const { ws: dev, opened: devOpen } = dial("/ws/device");
await devOpen;

dev.send(JSON.stringify({ cam }));
const telemetry = await nextMessage(ui);
assert.equal(telemetry.evt, "telemetry");
assert.equal(typeof telemetry.data.cam, "string");
assert.equal(telemetry.data.cam.length, cam.length);

ui.close();
dev.close();
console.log("cam relay OK", cam.length, "b64 chars");
process.exit(0);
