export type Dir = "up" | "down" | "left" | "right" | "stop";

export type RemoteEvent =
  | { evt: "device"; online: boolean }
  | { evt: "telemetry"; data: unknown };

const RECONNECT_DELAY_MS = 1000;
const WS_PATH = "/ws/ui";

function wsUrl(): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}${WS_PATH}`;
}

export class RemoteLink {
  private ws: WebSocket | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private onEvent: (e: RemoteEvent) => void,
    private onConn: (open: boolean) => void,
  ) {}

  connect(): void {
    this.open();
  }

  send(dir: Dir): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ dir }));
    }
  }

  private open(): void {
    const ws = new WebSocket(wsUrl());
    this.ws = ws;
    ws.onopen = () => this.onConn(true);
    ws.onclose = () => {
      this.onConn(false);
      this.retry ??= setTimeout(() => {
        this.retry = null;
        this.open();
      }, RECONNECT_DELAY_MS);
    };
    ws.onmessage = (m) => this.dispatch(m.data);
  }

  private dispatch(raw: unknown): void {
    if (typeof raw !== "string") return;
    try {
      this.onEvent(JSON.parse(raw) as RemoteEvent);
    } catch {
      return;
    }
  }
}
