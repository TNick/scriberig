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

it("provides a built-in public projection for the integrated route", () => {
  const projection = readFileSync(
    resolve(import.meta.dirname, "../../apps/scriberig/src/views/ProjectionView.tsx"),
    "utf8",
  );
  expect(projection).toContain("useObject");
  expect(projection).toContain("MuiObjectInspector");
  expect(projection).toContain("Select a semantic object to render.");
});
