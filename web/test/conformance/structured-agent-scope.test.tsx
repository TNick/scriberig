/** @file Verifies agent scope is represented as semantic selection. */
import { expect, it } from "vitest";
import { SelectionController } from "@scriberig/editor-core";
it("passes entity and snapshot handles", () => {
  const c = new SelectionController();
  c.set({ entityId: "e" as never, snapshotId: "s" as never });
  const scope = c.get();
  expect(scope).toMatchObject({ entityId: "e", snapshotId: "s" });
});
