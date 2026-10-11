/** @file Verifies the reference app has no private server imports. */
import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
it("uses public package boundaries", () => {
  const app = readFileSync(
    resolve(import.meta.dirname, "../../apps/scriberig/src/App.tsx"),
    "utf8",
  );
  expect(app).not.toMatch(/crates|server\/src|database|\/src\//);
});
