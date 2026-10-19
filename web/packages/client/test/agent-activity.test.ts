/** @file Verifies transient agent activity protocol mapping. */
import { describe, expect, it } from "vitest";
import { normalizeProtocolEvent } from "../src";

describe("agent activity event mapping", () => {
  it("normalizes safe transient activity metadata", () => {
    expect(
      normalizeProtocolEvent({
        kind: "AgentActivity",
        operation: { id: "op-1" },
        activity_kind: "ToolInvocation",
        activity_state: "Running",
        metadata: [
          ["branch", "branch-1"],
          ["summary", "Tool: search"],
        ],
      }),
    ).toEqual({
      kind: "agentActivity",
      operationId: "op-1",
      branchId: "branch-1",
      activityKind: "ToolInvocation",
      state: "Running",
      summary: "Tool: search",
    });
  });

  it("normalizes safe session model selection activity", () => {
    expect(
      normalizeProtocolEvent({
        kind: "SessionSelectionChanged",
        entity: { id: "session-1" },
        activity_kind: "ModelSelectionChanged",
        metadata: [["summary", "Model selected: Model A"]],
      }),
    ).toEqual({
      kind: "sessionSelectionChanged",
      sessionId: "session-1",
      activityKind: "ModelSelectionChanged",
      summary: "Model selected: Model A",
    });
  });

  it("normalizes durable plan activity with stable references", () => {
    expect(
      normalizeProtocolEvent({
        kind: "PlanActivity",
        entity: { id: "plan-1" },
        activity_kind: "PlanNodeCompleted",
        activity_state: "Completed",
        metadata: [
          ["session", "session-1"],
          ["node", "node-1"],
          ["summary", "Plan node completed"],
        ],
      }),
    ).toEqual({
      kind: "planActivity",
      sessionId: "session-1",
      planId: "plan-1",
      nodeId: "node-1",
      activityKind: "PlanNodeCompleted",
      state: "Completed",
      summary: "Plan node completed",
    });
  });
});
