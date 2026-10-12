/** @file Verifies immutable selection state can remain historical. */
import { expect, it } from "vitest";
import { SelectionController } from "@scriberig/editor-core";
import { RevisionCache } from "@scriberig/model";
it("keeps exact snapshot selection", () => {
  const c = new SelectionController();
  c.set({ entityId: "x" as never, snapshotId: "A" as never, revisionId: "x1" as never });
  expect(c.get()?.snapshotId).toBe("A");
});

it("reuses immutable revisions while live snapshot selection advances", () => {
  const cache = new RevisionCache();
  const revision = { entityId: "x", revisionId: "x1" } as never;
  cache.set(revision);
  const c = new SelectionController();
  c.set({ entityId: "x" as never, snapshotId: "A" as never, revisionId: "x1" as never });
  c.set({ entityId: "x" as never, snapshotId: "B" as never, revisionId: "x1" as never });
  expect(cache.get("x1")).toBe(revision);
  expect(c.get()).toMatchObject({ snapshotId: "B", revisionId: "x1" });
});
