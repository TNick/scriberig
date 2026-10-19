/** @file Verifies durable agent state and transient stream reduction. */

import { describe, expect, it } from "vitest";
import type { AgentEvent, BranchDescriptor } from "@scriberig/protocol";
import {
  createAgentWorkspaceState,
  reduceAgentEvent,
  markAgentWorkspaceStale,
  storeAgentSession,
  storeAgentPresentation,
  storeSharedAgentPresentation,
  storeAgentModelLibrary,
  agentPresentationPreset,
  resolveAgentPresentation,
} from "../src/agent-workspace";

const branch: BranchDescriptor = {
  branchId: "branch-1" as never,
  name: "root",
  status: "active",
  childCount: 0,
  activityCount: 0,
};

describe("agent workspace", () => {
  it("keeps default presentation choices client-local and independent", () => {
    const state = createAgentWorkspaceState();
    expect(state.presentation).toEqual({
      reasoning: "hidden",
      toolInvocation: "simple",
      toolOutput: "summary",
      prompt: "hidden",
    });
    expect(state.execution.revision).toBe(0);
  });

  it("resolves local preferences before shared values and defaults", () => {
    expect(
      resolveAgentPresentation(
        { prompt: "shown" },
        { reasoning: "shown", toolOutput: "full", prompt: "hidden" },
      ),
    ).toEqual({
      reasoning: "shown",
      toolInvocation: "simple",
      toolOutput: "full",
      prompt: "shown",
    });
  });

  it("expands verbosity presets into independent preference fields", () => {
    expect(agentPresentationPreset("minimal")).toEqual({
      reasoning: "hidden",
      toolInvocation: "simple",
      toolOutput: "hidden",
      prompt: "hidden",
    });
    expect(agentPresentationPreset("debug")).toEqual({
      reasoning: "shown",
      toolInvocation: "detailed",
      toolOutput: "full",
      prompt: "shown",
    });
  });

  it("keeps local display choices through authoritative session refresh", () => {
    const presentation = {
      reasoning: "shown" as const,
      toolInvocation: "detailed" as const,
      toolOutput: "full" as const,
      prompt: "shown" as const,
    };
    const library = { models: [], profiles: [] };
    let state = createAgentWorkspaceState();
    state = storeAgentPresentation(state, presentation);
    state = storeAgentModelLibrary(state, library);
    state = storeAgentSession(state, {
      session: { id: "session-1" as never },
      execution: { revision: 1 },
      branches: [],
      plans: [],
    });
    expect(state.presentation).toEqual(presentation);
    expect(state.modelLibrary).toBe(library);
  });

  it("applies shared defaults beneath local overrides immediately", () => {
    let state = createAgentWorkspaceState();
    state = storeSharedAgentPresentation(state, {
      reasoning: "shown",
      toolOutput: "full",
    });
    expect(state.presentation.reasoning).toBe("shown");
    expect(state.presentation.toolOutput).toBe("full");
    state = storeAgentPresentation(state, {
      reasoning: "hidden",
      toolInvocation: "simple",
      toolOutput: "summary",
      prompt: "hidden",
    });
    state = storeSharedAgentPresentation(state, {
      reasoning: "shown",
      toolOutput: "full",
      prompt: "shown",
    });
    expect(state.presentation.reasoning).toBe("hidden");
    expect(state.presentation.toolOutput).toBe("summary");
  });

  it("keeps local overrides field-specific as shared defaults change", () => {
    let state = createAgentWorkspaceState();
    state = storeSharedAgentPresentation(state, {
      reasoning: "shown",
      toolOutput: "full",
      prompt: "hidden",
    });
    state = storeAgentPresentation(state, { prompt: "shown" });
    state = storeSharedAgentPresentation(state, {
      reasoning: "hidden",
      toolOutput: "summary",
      prompt: "hidden",
    });

    expect(state.presentation).toEqual({
      reasoning: "hidden",
      toolInvocation: "simple",
      toolOutput: "summary",
      prompt: "shown",
    });
  });

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

  it("keeps live activity transient and clears it when the operation completes", () => {
    let state = createAgentWorkspaceState();
    state = reduceAgentEvent(state, {
      kind: "transientActivity",
      sequence: 1,
      operationId: "op-1" as never,
      branchId: "branch-1" as never,
      activityKind: "ToolInvocation",
      state: "Running",
      summary: "Tool: search",
    });
    expect(state.transientActivities["branch-1"]).toHaveLength(1);
    expect(state.activity["branch-1"]).toBeUndefined();
    state = reduceAgentEvent(state, {
      kind: "transientActivity",
      sequence: 1,
      operationId: "op-1" as never,
      branchId: "branch-1" as never,
      activityKind: "ToolInvocation",
      state: "Failed",
      summary: "out of order duplicate",
    });
    expect(state.transientActivities["branch-1"]?.[0]?.state).toBe("Running");
    state = reduceAgentEvent(state, {
      kind: "agentOutputCommitted",
      sequence: 2,
      operationId: "op-1" as never,
    });
    expect(state.transientActivities).toEqual({});
    expect(state.activity["branch-1"]).toBeUndefined();
  });

  it("retains safe selection activity and requests authoritative refresh", () => {
    const state = reduceAgentEvent(createAgentWorkspaceState(), {
      kind: "sessionSelectionChanged",
      sequence: 1,
      sessionId: "session-1" as never,
      activityKind: "ModelSelectionChanged",
      summary: "Model selected: Model A",
    });

    expect(state.selectionActivities).toHaveLength(1);
    expect(state.selectionActivities[0]?.summary).toBe("Model selected: Model A");
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
      execution: {
        revision: 3,
        modelProfile: "profile-a",
        model: "model-a",
        thinkingLevel: "deliberate",
      },
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
      plans: [],
    });
    expect(state.branches["branch-1"]?.status).toBe("completed");
    expect(state.branches["branch-1"]?.result?.resultId).toBe("result-1");
    expect(state.activity["branch-1"]?.items[0]?.summary).toBe("done");
    expect(state.execution).toEqual({
      revision: 3,
      modelProfile: "profile-a",
      model: "model-a",
      thinkingLevel: "deliberate",
    });
  });

  it("prefers the shared causal activity projection when present", () => {
    const state = storeAgentSession(createAgentWorkspaceState(), {
      session: { id: "session-1" as never },
      execution: {
        revision: 2,
        history: [
          {
            revision: 1,
            kind: "ModelSelectionChanged",
            summary: "Model selected: model-a",
            model: "model-a",
          },
          {
            revision: 2,
            kind: "ThinkingLevelChanged",
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
              id: "invoke-1",
              session: "session-1",
              sequence: 0,
              branch: "branch-1",
              kind: "ToolInvocation",
              state: "Completed",
              summary: "Tool: search",
              toolName: "search",
            },
            {
              id: "result-1",
              session: "session-1",
              sequence: 1,
              branch: "branch-1",
              parent: "invoke-1",
              kind: "ToolResult",
              state: "Completed",
              summary: "Tool completed",
            },
          ],
        },
      ],
      plans: [],
    });
    expect(state.activity["branch-1"]?.items).toMatchObject([
      { id: "invoke-1", kind: "tool", toolName: "search" },
      { id: "result-1", kind: "tool", parentId: "invoke-1" },
    ]);
  });

  it("marks durable agent state stale when the event stream disconnects", () => {
    const state = markAgentWorkspaceStale({
      ...createAgentWorkspaceState(),
      stale: false,
    });
    expect(state.stale).toBe(true);
  });
});
