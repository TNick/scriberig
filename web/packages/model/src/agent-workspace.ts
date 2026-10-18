/** @file Maintains durable agent branch state separately from transient streams. */

import type {
  BranchActivityPage,
  BranchChildrenPage,
  BranchDescriptor,
  Operation,
  SessionReadResponse,
  SessionExecutionSelection,
  ModelLibraryResponse,
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
  /** Discardable runtime activity keyed by durable branch identity. */
  transientActivities: Readonly<Record<string, readonly TransientAgentActivity[]>>;
  /** Bounded safe model and thinking selection changes observed from events. */
  selectionActivities: readonly SessionSelectionActivity[];
  /** Last accepted event sequence. */
  eventCursor: number;
  /** Whether a reconnect requires authoritative refresh. */
  stale: boolean;
  /** Current session model and thinking selection for future invocations. */
  execution: SessionExecutionSelection;
  /** Public model descriptors loaded from the shared client API. */
  modelLibrary?: ModelLibraryResponse;
  /** Client-local display choices that never alter session execution. */
  presentation: AgentPresentationPreferences;
  /** Shared user defaults received from the application. */
  sharedPresentation: AgentPresentationPreferenceLayer;
  /** Client-local overrides that take precedence over shared defaults. */
  localPresentation: AgentPresentationPreferenceLayer;
  /** Last safe error, when one was observed. */
  error?: string;
}

/** One transient safe status item received from the operation event stream. */
export interface TransientAgentActivity {
  /** Monotonic event sequence used for replay and deduplication. */
  sequence: number;
  /** Operation owning this live activity. */
  operationId: string;
  /** Durable branch associated with the runtime event. */
  branchId: string;
  /** Provider-neutral activity category. */
  activityKind: import("@scriberig/protocol").SessionActivityItem["kind"];
  /** Current lifecycle state. */
  state: import("@scriberig/protocol").SessionActivityItem["state"];
  /** Safe bounded display summary. */
  summary: string;
}

/** One safe model or thinking selection notification received by the client. */
export interface SessionSelectionActivity {
  /** Monotonic event cursor, present for transient notifications. */
  sequence?: number;
  /** Session execution mutation revision, present for durable history. */
  selectionRevision?: number;
  /** Stable session identity whose future execution selection changed. */
  sessionId: string;
  /** Typed model or thinking selection category. */
  activityKind: "ModelSelectionChanged" | "ThinkingLevelChanged";
  /** Bounded safe user-facing summary. */
  summary: string;
}

/** Independent display choices for already captured activity. */
export interface AgentPresentationPreferences {
  /** Whether safe reasoning summaries and status are visible. */
  reasoning: "hidden" | "shown";
  /** Whether tool parameters are compact or expanded. */
  toolInvocation: "simple" | "detailed";
  /** How much captured tool output is visible. */
  toolOutput: "hidden" | "summary" | "full";
  /** Whether captured safe model request data is visible. */
  prompt: "hidden" | "shown";
}

/** Partial client-local or shared preference layer. */
export type AgentPresentationPreferenceLayer = Partial<AgentPresentationPreferences>;

/** Named convenience preset that expands into independent display choices. */
export type AgentPresentationPreset = "minimal" | "normal" | "verbose" | "debug";

/** Creates default local presentation choices. @returns Default preferences. */
export function createAgentPresentationPreferences(): AgentPresentationPreferences {
  return {
    reasoning: "hidden",
    toolInvocation: "simple",
    toolOutput: "summary",
    prompt: "hidden",
  };
}

/** Expands one preset into independent presentation fields.
 * @param preset Named verbosity preset.
 * @returns Complete preference values without changing execution state.
 */
export function agentPresentationPreset(
  preset: AgentPresentationPreset,
): AgentPresentationPreferences {
  switch (preset) {
    case "minimal":
      return {
        reasoning: "hidden",
        toolInvocation: "simple",
        toolOutput: "hidden",
        prompt: "hidden",
      };
    case "normal":
      return {
        reasoning: "hidden",
        toolInvocation: "simple",
        toolOutput: "summary",
        prompt: "hidden",
      };
    case "verbose":
      return {
        reasoning: "shown",
        toolInvocation: "detailed",
        toolOutput: "full",
        prompt: "hidden",
      };
    case "debug":
      return {
        reasoning: "shown",
        toolInvocation: "detailed",
        toolOutput: "full",
        prompt: "shown",
      };
  }
}

/** Resolves client-local choices over shared choices and built-in defaults.
 * @param local Client-local overrides, when present.
 * @param shared Shared or user preference values, when present.
 * @returns Fully resolved display choices.
 */
export function resolveAgentPresentation(
  local: AgentPresentationPreferenceLayer = {},
  shared: AgentPresentationPreferenceLayer = {},
): AgentPresentationPreferences {
  const defaults = createAgentPresentationPreferences();
  return {
    reasoning: local.reasoning ?? shared.reasoning ?? defaults.reasoning,
    toolInvocation: local.toolInvocation ?? shared.toolInvocation ?? defaults.toolInvocation,
    toolOutput: local.toolOutput ?? shared.toolOutput ?? defaults.toolOutput,
    prompt: local.prompt ?? shared.prompt ?? defaults.prompt,
  };
}

/** Creates an empty agent workspace state. @returns Initial state. */
export function createAgentWorkspaceState(): AgentWorkspaceState {
  return {
    branches: {},
    children: {},
    activity: {},
    operations: {},
    transientStreams: {},
    transientActivities: {},
    selectionActivities: [],
    eventCursor: 0,
    stale: false,
    execution: { revision: 0 },
    presentation: createAgentPresentationPreferences(),
    sharedPresentation: {},
    localPresentation: {},
  };
}

export { reduceAgentEvent } from "./agent-workspace/events";

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
    const recordedActivity = branch.activity ?? [];
    branches[branch.id] = {
      branchId: branch.id,
      name: branch.title,
      status: branch.status.toLowerCase() as BranchDescriptor["status"],
      ...(branch.parent === undefined ? {} : { parentBranchId: branch.parent }),
      childCount: branch.children.length,
      activityCount: recordedActivity.length || branch.turns.length,
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
        [branch.id]:
          recordedActivity.length > 0
            ? {
                branchId: branch.id,
                items: recordedActivity.map((item) => ({
                  id: item.id,
                  ordinal: item.sequence,
                  kind:
                    item.kind === "HumanInput"
                      ? "human"
                      : item.kind === "BranchStarted"
                        ? "child"
                        : item.kind === "BranchCompleted"
                          ? "result"
                          : item.kind.startsWith("Tool")
                            ? "tool"
                            : "model",
                  activityKind: item.kind,
                  state: item.state,
                  summary: item.summary,
                  handles:
                    item.kind === "BranchStarted"
                      ? [item.id]
                      : item.durableRef === undefined
                        ? []
                        : [item.durableRef],
                  ...(item.parent === undefined ? {} : { parentId: item.parent }),
                  ...(item.model === undefined ? {} : { model: item.model }),
                  ...(item.thinkingLevel === undefined
                    ? {}
                    : { thinkingLevel: item.thinkingLevel }),
                  ...(item.toolName === undefined ? {} : { toolName: item.toolName }),
                  ...(item.toolArguments === undefined
                    ? {}
                    : { toolArguments: item.toolArguments }),
                })),
              }
            : {
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
  const selectionActivities = (session.execution.history ?? []).map((change) => ({
    selectionRevision: change.revision,
    sessionId: session.session.id,
    activityKind: change.kind,
    summary: change.summary,
  }));
  return {
    ...state,
    branches,
    execution: session.execution,
    selectionActivities,
    stale: false,
  };
}

/** Stores the safe public model library without persisting provider secrets.
 * @param state Current agent workspace state.
 * @param library Safe model descriptors and profiles.
 * @returns Updated workspace state.
 */
export function storeAgentModelLibrary(
  state: AgentWorkspaceState,
  library: ModelLibraryResponse,
): AgentWorkspaceState {
  return { ...state, modelLibrary: library };
}

/** Replaces client-local presentation choices without changing execution state.
 * @param state Current agent workspace state.
 * @param presentation Display choices for captured activity.
 * @returns Updated workspace state.
 */
export function storeAgentPresentation(
  state: AgentWorkspaceState,
  presentation: AgentPresentationPreferenceLayer,
): AgentWorkspaceState {
  const localPresentation = { ...state.localPresentation, ...presentation };
  return {
    ...state,
    localPresentation,
    presentation: resolveAgentPresentation(localPresentation, state.sharedPresentation),
  };
}

/** Stores shared defaults and resolves them beneath any client-local overrides.
 * @param state Current workspace state.
 * @param presentation Shared user defaults.
 * @returns Updated state with presentation choices resolved immediately.
 */
export function storeSharedAgentPresentation(
  state: AgentWorkspaceState,
  presentation: AgentPresentationPreferenceLayer,
): AgentWorkspaceState {
  return {
    ...state,
    sharedPresentation: presentation,
    presentation: resolveAgentPresentation(state.localPresentation, presentation),
  };
}
