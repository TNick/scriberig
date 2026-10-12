/** @file Maintains durable agent branch state separately from transient streams. */

import type {
  AgentEvent,
  BranchActivityPage,
  BranchChildrenPage,
  BranchDescriptor,
  Operation,
  SessionReadResponse,
} from "@scriberig/protocol";

/** Browser-owned state for one branch-aware agent workspace. */
export interface AgentWorkspaceState {
  /** Authoritative branch descriptors keyed by branch identity. */
  branches: Readonly<Record<string, BranchDescriptor>>;
  /** Loaded child pages keyed by parent branch identity. */
  children: Readonly<Record<string, BranchChildrenPage>>;
  /** Loaded activity pages keyed by branch identity. */
  activity: Readonly<Record<string, BranchActivityPage>>;
  /** Authoritative operation records keyed by operation identity. */
  operations: Readonly<Record<string, Operation>>;
  /** Discardable partial model text keyed by operation identity. */
  transientStreams: Readonly<Record<string, string>>;
  /** Last accepted event sequence. */
  eventCursor: number;
  /** Whether a reconnect requires authoritative refresh. */
  stale: boolean;
  /** Last safe error, when one was observed. */
  error?: string;
}

/** Creates an empty agent workspace state. @returns Initial state. */
export function createAgentWorkspaceState(): AgentWorkspaceState {
  return {
    branches: {},
    children: {},
    activity: {},
    operations: {},
    transientStreams: {},
    eventCursor: 0,
    stale: false,
  };
}

/** Applies one ordered agent event without promoting transient text to durable state.
 * @param state Current agent workspace state.
 * @param event Incoming provider-neutral event.
 * @returns Updated state.
 */
export function reduceAgentEvent(
  state: AgentWorkspaceState,
  event: AgentEvent,
): AgentWorkspaceState {
  if (event.sequence <= state.eventCursor) return state;
  const next = { ...state, eventCursor: event.sequence };
  const clearTransient = (operationId: string): Readonly<Record<string, string>> => {
    const streams = { ...state.transientStreams };
    delete streams[operationId];
    return streams;
  };
  switch (event.kind) {
    case "branchCreated":
      return {
        ...next,
        branches: { ...state.branches, [event.branch.branchId]: event.branch },
      };
    case "branchChanged":
      return { ...next, stale: true };
    case "modelTextDelta":
      return {
        ...next,
        transientStreams: {
          ...state.transientStreams,
          [event.operationId]: `${state.transientStreams[event.operationId] ?? ""}${event.text}`,
        },
      };
    case "agentOutputCommitted": {
      const streams = { ...state.transientStreams };
      delete streams[event.operationId];
      return { ...next, transientStreams: streams, stale: true };
    }
    case "operationProgress":
      return { ...next, stale: true };
    case "operationCompleted":
      return { ...next, transientStreams: clearTransient(event.operationId), stale: true };
    case "operationCancelled":
      return { ...next, transientStreams: clearTransient(event.operationId), stale: true };
    case "operationFailed":
      return {
        ...next,
        transientStreams: clearTransient(event.operationId),
        stale: true,
        error: event.failure.message,
      };
  }
}

/** Replaces an authoritative branch-child page. @param state Current state. @param page Loaded page. @returns Updated state. */
export function storeBranchChildren(
  state: AgentWorkspaceState,
  page: BranchChildrenPage,
): AgentWorkspaceState {
  return {
    ...state,
    children: { ...state.children, [page.branchId]: page },
    stale: false,
  };
}

/** Replaces an authoritative branch-activity page. @param state Current state. @param page Loaded page. @returns Updated state. */
export function storeBranchActivity(
  state: AgentWorkspaceState,
  page: BranchActivityPage,
): AgentWorkspaceState {
  return {
    ...state,
    activity: { ...state.activity, [page.branchId]: page },
    stale: false,
  };
}

/** Stores an authoritative operation in the durable agent workspace state.
 * @param state Current agent workspace state.
 * @param operation Operation returned by the public client.
 * @returns Updated workspace state.
 */
export function storeAgentOperation(
  state: AgentWorkspaceState,
  operation: Operation,
): AgentWorkspaceState {
  return {
    ...state,
    operations: { ...state.operations, [operation.operationId]: operation },
    stale: false,
  };
}

/** Marks agent state stale after an event-stream disconnect.
 * @param state Current agent workspace state.
 * @returns State requiring authoritative refresh.
 */
export function markAgentWorkspaceStale(state: AgentWorkspaceState): AgentWorkspaceState {
  return { ...state, stale: true };
}

/** Stores compact durable session branches without importing activity transcripts.
 * @param state Current agent workspace state.
 * @param session Session response returned by the public client.
 * @returns Updated workspace state.
 */
export function storeAgentSession(
  state: AgentWorkspaceState,
  session: SessionReadResponse,
): AgentWorkspaceState {
  const branches = { ...state.branches };
  for (const branch of session.branches) {
    branches[branch.id] = {
      branchId: branch.id,
      name: branch.title,
      status: branch.status.toLowerCase() as BranchDescriptor["status"],
      ...(branch.parent === undefined ? {} : { parentBranchId: branch.parent }),
      childCount: branch.children.length,
      activityCount: branch.turns.length,
      ...(branch.result === undefined
        ? {}
        : {
            result: {
              branchId: branch.id,
              resultId: branch.result.id,
              handles: branch.result.references,
            },
          }),
    };
    state = {
      ...state,
      activity: {
        ...state.activity,
        [branch.id]: {
          branchId: branch.id,
          items: branch.turns.map((turn) => ({
            id: turn.id,
            ordinal: turn.ordinal,
            kind: turn.hasAgentOutput ? "model" : turn.toolInvocation ? "tool" : "human",
            summary: turn.agentOutput ?? (turn.hasHumanInput ? "Human input" : "Activity"),
            handles: [
              ...(turn.toolInvocation === undefined ? [] : [turn.toolInvocation]),
              ...(turn.toolResult === undefined ? [] : [turn.toolResult]),
            ],
          })),
        },
      },
    };
  }
  return { ...state, branches, stale: false };
}
