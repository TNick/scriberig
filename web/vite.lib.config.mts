/** @file Defines the shared library build for reusable Scriberig packages. */

import { isAbsolute, resolve } from "node:path";
import { defineConfig } from "vite";

/** Builds the package entrypoint as a portable ESM library. */
export default defineConfig({
  build: {
    emptyOutDir: true,
    lib: {
      entry: resolve(process.cwd(), "src/index.ts"),
      formats: ["es"],
      fileName: () => "index.js",
    },
    outDir: resolve(process.cwd(), "dist"),
    rollupOptions: {
      external: (id) => !id.startsWith(".") && !isAbsolute(id),
    },
  },
});
