// Regression test: relay must deliver a camera frame that arrives FRAGMENTED,
// the way esp_websocket_client actually sends it (1 KB chunks, last has FIN).
// Plain WebSocket (cam.mjs) cannot reproduce this — it never fragments.
import assert from "node:assert";
import net from "node:net";
import crypto from "node:crypto";

const HOST = process.env.HOST ?? "localhost";
const PORT = Number(process.env.PORT ?? 8080);
const CHUNK = 1024; // esp_cam_tank CLIENT_BUFFER_SIZE
const TIMEOUT_MS = 5000;
const CAM_BYTES = 24 * 1024;

function frame(fin, opcode, payload) {
  const mask = crypto.randomBytes(4);
  const len = payload.length;
  let header;
  if (len < 126) {
    header = Buffer.alloc(2);
    header[1] = 0x80 | len;
  } else if (len < 65536) {
    header = Buffer.alloc(4);
    header[1] = 0x80 | 126;
    header.writeUInt16BE(len, 2);
  } else {
    header = Buffer.alloc(10);
    header[1] = 0x80 | 127;
    header.writeBigUInt64BE(BigInt(len), 2);
  }
  header[0] = (fin ? 0x80 : 0) | opcode;
  const body = Buffer.from(payload);
  for (let i = 0; i < len; i += 1) body[i] ^= mask[i & 3];
  return Buffer.concat([header, mask, body]);
}

function connect(path) {
  const socket = net.connect(PORT, HOST);
  const key = crypto.randomBytes(16).toString("base64");
  socket.write(
    `GET ${path} HTTP/1.1\r\nHost: ${HOST}:${PORT}\r\nUpgrade: websocket\r\n` +
      `Connection: Upgrade\r\nSec-WebSocket-Key: ${key}\r\nSec-WebSocket-Version: 13\r\n\r\n`,
  );
  return new Promise((resolve, reject) => {
    let buf = Buffer.alloc(0);
    socket.on("error", reject);
    socket.on("data", (chunk) => {
      buf = Buffer.concat([buf, chunk]);
      const idx = buf.indexOf("\r\n\r\n");
      if (idx === -1) return;
      const head = buf.subarray(0, idx).toString();
      socket.pause();
      socket.removeAllListeners("data");
      if (!head.includes("101")) reject(new Error(`upgrade failed: ${head.split("\r\n")[0]}`));
      else resolve({ socket, pending: buf.subarray(idx + 4) });
    });
  });
}

function nextText({ socket }, pending) {
  return new Promise((resolve, reject) => {
    let buf = pending;
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("message timeout"));
    }, TIMEOUT_MS);
    socket.on("data", (chunk) => {
      buf = Buffer.concat([buf, chunk]);
      if (buf.length < 2) return;
      const opcode = buf[0] & 0x0f;
      let len = buf[1] & 0x7f;
      let off = 2;
      if (len === 126) {
        if (buf.length < 4) return;
        len = buf.readUInt16BE(2);
        off = 4;
      } else if (len === 127) {
        if (buf.length < 10) return;
        len = Number(buf.readBigUInt64BE(2));
        off = 10;
      }
      if (buf.length < off + len) return;
      socket.pause();
      socket.removeAllListeners("data");
      clearTimeout(timer);
      if (opcode !== 1) {
        socket.destroy();
        reject(new Error(`unexpected opcode ${opcode}`));
        return;
      }
      resolve(buf.subarray(off, off + len).toString());
    });
    socket.resume();
  });
}

async function nextTelemetry(ui, pending) {
  for (let i = 0; i < 5; i += 1) {
    const msg = JSON.parse(await nextText(ui, pending));
    pending = Buffer.alloc(0);
    if (msg.evt === "telemetry") return msg;
  }
  throw new Error("no telemetry within 5 messages");
}

const jpeg = Buffer.alloc(CAM_BYTES);
jpeg[0] = 0xff;
jpeg[1] = 0xd8;
jpeg[2] = 0xff;
jpeg[3] = 0xd9;
const cam = jpeg.toString("base64");
const msg = Buffer.from(JSON.stringify({ cam }));

const ui = await connect("/ws/ui");
await nextText(ui, ui.pending); // hello: {"evt":"device",...}

const dev = await connect("/ws/device");
let first = true;
for (let off = 0; off < msg.length; off += CHUNK) {
  const end = Math.min(off + CHUNK, msg.length);
  dev.socket.write(frame(end === msg.length, first ? 0x1 : 0x0, msg.subarray(off, end)));
  first = false;
}

const telemetry = await nextTelemetry(ui, Buffer.alloc(0));
assert.equal(telemetry.evt, "telemetry");
assert.equal(typeof telemetry.data.cam, "string");
assert.equal(telemetry.data.cam.length, cam.length);

ui.socket.end();
dev.socket.end();
console.log(`cam frag relay OK: ${cam.length} b64 chars in ${Math.ceil(msg.length / CHUNK)} fragments`);
process.exit(0);
