/** @file Configures the reference Scriberig browser application build. */

import { defineConfig } from "vite";

// Proxy browser API calls to the local Rust development server.
export default defineConfig({
  server: {
    proxy: {
      "/api": "http://127.0.0.1:7337",
    },
  },
});
