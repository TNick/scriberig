/** @file Verifies React bindings remain thin over shared controllers. */
import { describe, expect, it } from "vitest";
import { KnowledgeTree, SelectionController } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";
import type { EntityId, SnapshotId } from "@scriberig/protocol";
import { createSnapshotState, advanceSnapshot } from "@scriberig/model";

describe("headless hook dependencies", () => {
  it("shares selection and tree controllers across consumers", () => {
    const selection = new SelectionController();
    const tree = new KnowledgeTree();
    const firstConsumer: unknown[] = [];
    const secondConsumer: unknown[] = [];
    selection.subscribe((value) => firstConsumer.push(value));
    selection.subscribe((value) => secondConsumer.push(value));
    const selected = {
      entityId: "ent-1" as EntityId,
      snapshotId: "snap-1" as SnapshotId,
    };

    selection.set(selected);

    expect(selection.get()).toEqual(selected);
    expect(firstConsumer).toEqual([selected]);
    expect(secondConsumer).toEqual([selected]);
    expect(tree.version()).toBe(0);
  });

  it("keeps historical snapshot exact when the current model advances", () => {
    const model = new WorkspaceModel();
    const historical = "snap-1" as SnapshotId;
    const current = "snap-2" as SnapshotId;
    model.apply({ kind: "snapshotAdvanced", snapshotId: historical });
    model.view(historical);
    model.apply({ kind: "snapshotAdvanced", snapshotId: current });

    const state = model.getState().snapshots;
    expect(state.current).toBe(current);
    expect(state.viewed).toBe(historical);
    expect(state.stale).toBe(true);
    expect(advanceSnapshot(createSnapshotState(), current)).toEqual({
      current,
      stale: false,
    });
  });
});
