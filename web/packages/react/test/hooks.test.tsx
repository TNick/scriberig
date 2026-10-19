/** @file Verifies React bindings remain thin over shared controllers. */
import { describe, expect, it } from "vitest";
import { KnowledgeTree, SelectionController } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";
import type { EntityId, SnapshotId } from "@scriberig/protocol";
import { createSnapshotState, advanceSnapshot } from "@scriberig/model";
import { useMigrationReview } from "../src/use-migration-review";
import { createAgentExecutionSelectionController } from "../src/use-agent-execution-selection";

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

  it("returns host-owned migration review actions", () => {
    const apply = () => undefined;
    const reject = () => undefined;
    const review = useMigrationReview(
      { proposalId: "proposal-1", stale: false, conflict: false },
      { apply, reject },
    );

    expect(review.state.proposalId).toBe("proposal-1");
    expect(review.apply).toBe(apply);
    expect(review.reject).toBe(reject);
  });

  it("uses the latest session revision and refreshes shared selection state", async () => {
    const model = new WorkspaceModel();
    const sessionId = "session-1" as never;
    const requests: Array<[string, number, string]> = [];
    let revision = 4;
    const client = {
      getSession: async () => ({
        session: { id: sessionId },
        branches: [],
        plans: [],
        execution: { revision },
      }),
      setSessionModel: async (_session: unknown, expected: number, profile: string) => {
        requests.push(["model", expected, profile]);
        revision += 1;
        return { revision };
      },
      setSessionThinkingLevel: async (_session: unknown, expected: number, level: string) => {
        requests.push(["thinking", expected, level]);
        revision += 1;
        return { revision };
      },
    } as never;
    model.setAgentSession({
      session: { id: sessionId },
      branches: [],
      plans: [],
      execution: { revision: 4 },
    });
    const controller = createAgentExecutionSelectionController(client, model, sessionId);

    await controller.selectModel("profile-b");
    await controller.selectThinkingLevel("deep");

    expect(requests).toEqual([
      ["model", 4, "profile-b"],
      ["thinking", 5, "deep"],
    ]);
    expect(model.getAgentState().execution.revision).toBe(6);
  });
});
