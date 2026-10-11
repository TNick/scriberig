/** @file Verifies semantic selection remains a shared controller value. */
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
