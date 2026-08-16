import "./style.css";
import { RemoteLink, type Dir, type RemoteEvent } from "./ws";

const KEY_DIRS: Readonly<Record<string, Dir>> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Space: "stop",
};

const STOP: Dir = "stop";
const ACTIVE_CLASS = "active";

const statusEl = document.getElementById("status")!;
const dotEl = document.getElementById("dot")!;
const telemetryEl = document.getElementById("telemetry")!;
const camEl = document.getElementById("cam") as HTMLImageElement;

let wsOpen = false;
let deviceOnline = false;

const link = new RemoteLink(onEvent, onConn);

function onConn(open: boolean): void {
  wsOpen = open;
  renderStatus();
}

function isCamFrame(data: unknown): data is { cam: string } {
  return (
    typeof data === "object" &&
    data !== null &&
    "cam" in data &&
    typeof (data as { cam: unknown }).cam === "string"
  );
}

function onEvent(e: RemoteEvent): void {
  if (e.evt === "device") {
    deviceOnline = e.online;
    renderStatus();
  } else if (isCamFrame(e.data)) {
    camEl.src = `data:image/jpeg;base64,${e.data.cam}`;
    camEl.hidden = false;
  } else {
    telemetryEl.textContent = JSON.stringify(e.data, null, 2);
    telemetryEl.hidden = false;
  }
}

function renderStatus(): void {
  dotEl.className = wsOpen ? (deviceOnline ? "dot online" : "dot warn") : "dot offline";
  statusEl.textContent = !wsOpen
    ? "server: disconnected"
    : deviceOnline
      ? "tank: online"
      : "tank: offline";
}

function byDir(dir: Dir): HTMLButtonElement | null {
  return document.querySelector(`#pad button[data-dir="${dir}"]`);
}

function setHighlight(dir: Dir, on: boolean): void {
  byDir(dir)?.classList.toggle(ACTIVE_CLASS, on);
}

function bindPad(): void {
  for (const btn of document.querySelectorAll<HTMLButtonElement>("#pad button")) {
    const dir = btn.dataset.dir as Dir;
    btn.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      btn.setPointerCapture(e.pointerId);
      setHighlight(dir, true);
      link.send(dir);
    });
    const release = () => {
      setHighlight(dir, false);
      link.send(STOP);
    };
    btn.addEventListener("pointerup", release);
    btn.addEventListener("pointercancel", release);
  }
}

function bindKeys(): void {
  const held = new Set<string>();
  window.addEventListener("keydown", (e) => {
    const dir = KEY_DIRS[e.code];
    if (!dir || held.has(e.code)) return;
    e.preventDefault();
    held.add(e.code);
    setHighlight(dir, true);
    link.send(dir);
  });
  window.addEventListener("keyup", (e) => {
    const dir = KEY_DIRS[e.code];
    if (dir && held.delete(e.code)) {
      setHighlight(dir, false);
      link.send(STOP);
    }
  });
}

bindPad();
bindKeys();
renderStatus();
link.connect();
