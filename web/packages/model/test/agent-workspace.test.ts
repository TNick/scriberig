/** @file Verifies durable agent state and transient stream reduction. */

import { describe, expect, it } from "vitest";
import type { AgentEvent, BranchDescriptor } from "@scriberig/protocol";
import {
  createAgentWorkspaceState,
  reduceAgentEvent,
  markAgentWorkspaceStale,
  storeAgentSession,
} from "../src/agent-workspace";

const branch: BranchDescriptor = {
  branchId: "branch-1" as never,
  name: "root",
  status: "active",
  childCount: 0,
  activityCount: 0,
};

describe("agent workspace", () => {
  it("keeps model deltas transient and clears them on commit", () => {
    let state = createAgentWorkspaceState();
    const delta: AgentEvent = {
      kind: "modelTextDelta",
      sequence: 1,
      operationId: "op-1" as never,
      text: "partial",
    };
    state = reduceAgentEvent(state, delta);
    expect(state.transientStreams["op-1"]).toBe("partial");
    state = reduceAgentEvent(state, {
      kind: "agentOutputCommitted",
      sequence: 2,
      operationId: "op-1" as never,
    });
    expect(state.transientStreams["op-1"]).toBeUndefined();
    expect(state.stale).toBe(true);
  });

  it("ignores duplicate or older event sequences", () => {
    let state = createAgentWorkspaceState();
    state = reduceAgentEvent(state, { kind: "branchCreated", sequence: 2, branch });
    state = reduceAgentEvent(state, {
      kind: "modelTextDelta",
      sequence: 1,
      operationId: "op-1" as never,
      text: "old",
    });
    expect(state.transientStreams).toEqual({});
    expect(state.eventCursor).toBe(2);
  });

  it("discards transient output when an operation reaches a terminal state", () => {
    let state = createAgentWorkspaceState();
    state = reduceAgentEvent(state, {
      kind: "modelTextDelta",
      sequence: 1,
      operationId: "op-1" as never,
      text: "partial",
    });
    state = reduceAgentEvent(state, {
      kind: "operationCompleted",
      sequence: 2,
      operationId: "op-1" as never,
    });
    expect(state.transientStreams).toEqual({});
    expect(state.stale).toBe(true);
  });

  it("projects a durable session read into branches and activity", () => {
    const state = storeAgentSession(createAgentWorkspaceState(), {
      session: { id: "session-1" as never },
      branches: [
        {
          id: "branch-1" as never,
          title: "root",
          status: "Completed",
          children: [],
          turns: [
            {
              id: "turn-1",
              ordinal: 1,
              hasHumanInput: true,
              hasAgentOutput: true,
              agentOutput: "done",
            },
          ],
          result: { id: "result-1", references: ["ent-1"] },
        },
      ],
    });
    expect(state.branches["branch-1"]?.status).toBe("completed");
    expect(state.branches["branch-1"]?.result?.resultId).toBe("result-1");
    expect(state.activity["branch-1"]?.items[0]?.summary).toBe("done");
  });

  it("marks durable agent state stale when the event stream disconnects", () => {
    const state = markAgentWorkspaceStale({
      ...createAgentWorkspaceState(),
      stale: false,
    });
    expect(state.stale).toBe(true);
  });
});
