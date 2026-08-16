import { defineConfig } from "vite";

const BACKEND_ORIGIN = "http://localhost:8080";
const WS_PREFIX = "/ws";

export default defineConfig({
  server: {
    proxy: {
      [WS_PREFIX]: { target: BACKEND_ORIGIN, ws: true },
    },
  },
});
