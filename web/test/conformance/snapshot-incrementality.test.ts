/** @file Verifies immutable selection state can remain historical. */
import { expect, it } from "vitest";
import { SelectionController } from "@scriberig/editor-core";
it("keeps exact snapshot selection", () => {
  const c = new SelectionController();
  c.set({ entityId: "x" as never, snapshotId: "A" as never, revisionId: "x1" as never });
  expect(c.get()?.snapshotId).toBe("A");
});
