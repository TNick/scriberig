/** @file Configures root web conformance tests. */
import { defineConfig } from "vitest/config";
import { resolve } from "node:path";
export default defineConfig({
  resolve: {
    alias: {
      "@scriberig/editor-core": resolve(__dirname, "packages/editor-core/src/index.ts"),
      "@scriberig/model": resolve(__dirname, "packages/model/src/index.ts"),
    },
  },
  test: { include: ["test/conformance/**/*.test.ts", "test/conformance/**/*.test.tsx"] },
});
