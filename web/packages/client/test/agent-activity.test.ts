/** @file Verifies transient agent activity protocol mapping. */
import { describe, expect, it } from "vitest";
import { normalizeProtocolEvent } from "../src";

describe("agent activity event mapping", () => {
  it("normalizes safe transient activity metadata", () => {
    expect(
      normalizeProtocolEvent({
        kind: "AgentActivity",
        operation: { id: "op-1" },
        metadata: [
          ["branch", "branch-1"],
          ["activity_kind", "toolInvocation"],
          ["state", "running"],
          ["summary", "Tool: search"],
        ],
      }),
    ).toEqual({
      kind: "agentActivity",
      operationId: "op-1",
      branchId: "branch-1",
      activityKind: "toolInvocation",
      state: "running",
      summary: "Tool: search",
    });
  });

  it("normalizes safe session model selection activity", () => {
    expect(
      normalizeProtocolEvent({
        kind: "SessionSelectionChanged",
        entity: { id: "session-1" },
        metadata: [
          ["activity_kind", "modelSelection"],
          ["summary", "Model selected: Model A"],
        ],
      }),
    ).toEqual({
      kind: "sessionSelectionChanged",
      sessionId: "session-1",
      activityKind: "modelSelection",
      summary: "Model selected: Model A",
    });
  });
});
