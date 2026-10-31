/** @file Verifies semantic selection remains a shared controller value. */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { SelectionController } from "@scriberig/editor-core";
it("preserves semantic identity while projection metadata changes", () => {
  const controller = new SelectionController();
  const selection = { entityId: "entity" as never, snapshotId: "snapshot" as never };
  controller.set(selection);
  controller.updateProjection({ key: "preview", location: "line:1" });
  expect(controller.get()?.entityId).toBe(selection.entityId);
  expect(controller.get()?.projection?.location).toBe("line:1");
});

it("renders shared selection context around every integrated route", () => {
  const app = readFileSync(
    resolve(import.meta.dirname, "../../apps/scriberig/src/App.tsx"),
    "utf8",
  );
  const summary = app.indexOf("<SharedSelectionSummary />");
  const firstRoute = app.indexOf("<AppRouteContent");
  expect(summary).toBeGreaterThan(app.indexOf("<ScriberigProvider"));
  expect(summary).toBeGreaterThan(-1);
  expect(firstRoute).toBeGreaterThan(summary);
});
