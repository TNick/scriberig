/** @file Reduces monotonic live events into transient agent workspace state. */

import type { AgentEvent } from "@scriberig/protocol";
import type { AgentWorkspaceState, TransientAgentActivity } from "../agent-workspace";

/** Applies one ordered agent event without promoting transient activity to durable state.
 * @param state Current agent workspace state.
 * @param event Incoming provider-neutral event.
 * @returns Updated workspace state.
 */
export function reduceAgentEvent(
  state: AgentWorkspaceState,
  event: AgentEvent,
): AgentWorkspaceState {
  if (event.sequence <= state.eventCursor) return state;
  const next = { ...state, eventCursor: event.sequence };
  if (state.eventCursor > 0 && event.sequence > state.eventCursor + 1) {
    return { ...next, stale: true };
  }
  const clearTransient = (operationId: string): Readonly<Record<string, string>> => {
    const streams = { ...state.transientStreams };
    delete streams[operationId];
    return streams;
  };
  const clearActivities = (
    operationId: string,
  ): Readonly<Record<string, readonly TransientAgentActivity[]>> => {
    const activities: Record<string, readonly TransientAgentActivity[]> = {};
    for (const [branchId, items] of Object.entries(state.transientActivities)) {
      const remaining = items.filter((item) => item.operationId !== operationId);
      if (remaining.length > 0) activities[branchId] = remaining;
    }
    return activities;
  };
  switch (event.kind) {
    case "branchCreated":
      return {
        ...next,
        branches: { ...state.branches, [event.branch.branchId]: event.branch },
      };
    case "branchChanged":
      return { ...next, stale: true };
    case "sessionSelectionChanged":
      return {
        ...next,
        selectionActivities: [...state.selectionActivities, event].slice(-50),
        stale: true,
      };
    case "modelTextDelta":
      return {
        ...next,
        transientStreams: {
          ...state.transientStreams,
          [event.operationId]: `${state.transientStreams[event.operationId] ?? ""}${event.text}`,
        },
      };
    case "transientActivity": {
      const items = state.transientActivities[event.branchId] ?? [];
      if (items.some((item) => item.sequence === event.sequence)) return next;
      return {
        ...next,
        transientActivities: {
          ...state.transientActivities,
          [event.branchId]: [...items, event].slice(-100),
        },
      };
    }
    case "agentOutputCommitted": {
      const streams = { ...state.transientStreams };
      delete streams[event.operationId];
      return {
        ...next,
        transientStreams: streams,
        transientActivities: clearActivities(event.operationId),
        stale: true,
      };
    }
    case "operationProgress":
      return { ...next, stale: true };
    case "operationCompleted":
    case "operationCancelled":
      return {
        ...next,
        transientStreams: clearTransient(event.operationId),
        transientActivities: clearActivities(event.operationId),
        stale: true,
      };
    case "operationFailed":
      return {
        ...next,
        transientStreams: clearTransient(event.operationId),
        transientActivities: clearActivities(event.operationId),
        stale: true,
        error: event.failure.message,
      };
  }
}
