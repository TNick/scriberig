/** @file Verifies session reload restores durable activity and selection history. */

import { describe, expect, it } from "vitest";
import {
  createAgentWorkspaceState,
  reduceAgentEvent,
  storeAgentSession,
} from "../src/agent-workspace";

describe("agent workspace recovery", () => {
  it("rebuilds durable activity and selection history after transient activity is lost", () => {
    const session = {
      session: { id: "session-1" as never },
      execution: {
        revision: 2,
        history: [
          {
            revision: 1,
            kind: "ModelSelectionChanged" as const,
            summary: "Model selected: model-a",
            model: "model-a",
          },
          {
            revision: 2,
            kind: "ThinkingLevelChanged" as const,
            summary: "Thinking level selected: steady",
            model: "model-a",
            thinkingLevel: "steady",
          },
        ],
      },
      branches: [
        {
          id: "branch-1" as never,
          title: "root",
          status: "Active" as const,
          children: [],
          turns: [],
          activity: [
            {
              id: "model-1",
              session: "session-1",
              sequence: 0,
              branch: "branch-1",
              kind: "ModelInvocation" as const,
              state: "Completed" as const,
              summary: "Model request: model-a",
            },
            {
              id: "invoke-1",
              session: "session-1",
              sequence: 1,
              branch: "branch-1",
              parent: "model-1",
              kind: "ToolInvocation" as const,
              state: "Completed" as const,
              summary: "Tool: search",
              toolName: "search",
            },
            {
              id: "result-1",
              session: "session-1",
              sequence: 2,
              branch: "branch-1",
              parent: "invoke-1",
              kind: "ToolResult" as const,
              state: "Completed" as const,
              summary: "Tool completed",
            },
            {
              id: "output-1",
              session: "session-1",
              sequence: 3,
              branch: "branch-1",
              parent: "model-1",
              kind: "AssistantOutput" as const,
              state: "Completed" as const,
              summary: "Recovered final result",
            },
          ],
        },
      ],
    };
    const loaded = storeAgentSession(createAgentWorkspaceState(), session);
    const live = reduceAgentEvent(loaded, {
      kind: "transientActivity",
      sequence: 1,
      operationId: "op-1" as never,
      branchId: "branch-1" as never,
      activityKind: "ToolResult",
      state: "Completed",
      summary: "Tool search completed",
    });
    const liveReasoning = reduceAgentEvent(live, {
      kind: "transientActivity",
      sequence: 2,
      operationId: "op-1" as never,
      branchId: "branch-1" as never,
      activityKind: "ReasoningSummary",
      state: "Completed",
      summary: "Safe live summary",
    });
    const reloaded = storeAgentSession(createAgentWorkspaceState(), session);

    expect(liveReasoning.transientActivities["branch-1"]).toHaveLength(2);
    expect(reloaded.transientActivities).toEqual({});
    expect(reloaded.selectionActivities).toMatchObject([
      { selectionRevision: 1, activityKind: "ModelSelectionChanged" },
      { selectionRevision: 2, activityKind: "ThinkingLevelChanged" },
    ]);
    expect(reloaded.activity["branch-1"]?.items).toMatchObject([
      { id: "model-1", kind: "model" },
      { id: "invoke-1", kind: "tool", summary: "Tool: search", parentId: "model-1" },
      { id: "result-1", kind: "tool", parentId: "invoke-1" },
      { id: "output-1", kind: "model", summary: "Recovered final result" },
    ]);
  });
});
