/** @file Configures the reference Scriberig browser application build. */

import { defineConfig, loadEnv } from "vite";

/** Configures API proxying for the active local service port. */
export default defineConfig(({ mode }) => {
  const { SCRIBE_API_PORT = "7337" } = loadEnv(mode, ".", "");
  return {
    server: {
      allowedHosts: ["scriberig-dev.advaita.ro"],
      proxy: {
        "/api": `http://127.0.0.1:${SCRIBE_API_PORT}`,
      },
    },
  };
});
