/** @file Normalizes and projects durable session reads for browser clients. */
import type {
  BranchActivityPage,
  BranchChildrenPage,
  BranchId,
  BranchResultReference,
  SessionId,
  SessionReadResponse,
  SessionActivityItem,
} from "@scriberig/protocol";

/** Normalizes a wire session response. @param value Unknown response. @returns Public session tree. @throws For invalid responses. */
export function normalizeSession(value: unknown): SessionReadResponse {
  if (typeof value !== "object" || value === null)
    throw new Error("server returned an invalid session");
  const record = value as Record<string, unknown>;
  const session = record.session as Record<string, unknown> | undefined;
  if (typeof session?.id !== "string") throw new Error("server returned an incomplete session");
  return {
    session: { id: session.id as SessionId },
    branches: (Array.isArray(record.branches) ? record.branches : []).map(normalizeBranch),
    execution: normalizeExecution(record.execution),
  };
}

/** Finds one branch descriptor by stable branch identity.
 * @param session Authoritative session response.
 * @param branchId Requested branch identity.
 * @returns The corresponding lightweight branch descriptor.
 * @throws When the branch is absent from the session.
 */
export function branchFromSession(
  session: SessionReadResponse,
  branchId: BranchId,
): BranchChildrenPage["items"][number] {
  const branch = session.branches.find((candidate) => candidate.id === branchId);
  if (branch === undefined) throw new Error("branch is not present in session");
  return branchDescriptor(branch);
}

/** Projects child descriptors without loading their activity.
 * @param session Authoritative session response.
 * @param branchId Parent branch identity.
 * @returns Child descriptors and their parent identity.
 * @throws When the parent branch is absent from the session.
 */
export function childrenFromSession(
  session: SessionReadResponse,
  branchId: BranchId,
): BranchChildrenPage {
  const branch = session.branches.find((candidate) => candidate.id === branchId);
  if (branch === undefined) throw new Error("branch is not present in session");
  const children = new Set(branch.children);
  return {
    branchId,
    items: session.branches.filter((candidate) => children.has(candidate.id)).map(branchDescriptor),
  };
}

/** Projects one branch activity page.
 * @param session Authoritative session response.
 * @param branchId Branch whose activity is requested.
 * @returns Bounded branch activity.
 * @throws When the branch is absent from the session.
 */
export function activityFromSession(
  session: SessionReadResponse,
  branchId: BranchId,
): BranchActivityPage {
  const branch = session.branches.find((candidate) => candidate.id === branchId);
  if (branch === undefined) throw new Error("branch is not present in session");
  return branchActivity(branch);
}

/** Resolves one durable branch result reference.
 * @param session Authoritative session response.
 * @param resultId Requested stable result identity.
 * @returns Branch and result identities with referenced handles.
 * @throws When the result is absent from the session.
 */
export function resultFromSession(
  session: SessionReadResponse,
  resultId: string,
): BranchResultReference {
  const branch = session.branches.find((candidate) => candidate.result?.id === resultId);
  if (branch?.result === undefined) throw new Error("branch result is not present in session");
  return {
    branchId: branch.id,
    resultId: branch.result.id,
    handles: branch.result.references,
  };
}

/** Normalizes persisted model selection from a session response.
 * @param value Unknown execution selection.
 * @returns Safe selection with defaults for older servers.
 */
function normalizeExecution(value: unknown): SessionReadResponse["execution"] {
  const record =
    typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  const modelProfile = record.modelProfile ?? record.model_profile;
  const thinkingLevel = record.thinkingLevel ?? record.thinking_level;
  return {
    revision: typeof record.revision === "number" ? record.revision : 0,
    ...(typeof modelProfile === "string" ? { modelProfile } : {}),
    ...(typeof record.model === "string" ? { model: record.model } : {}),
    ...(typeof thinkingLevel === "string" ? { thinkingLevel } : {}),
  };
}

/** Normalizes a wire branch. @param value Unknown branch. @returns Public branch. */
function normalizeBranch(value: unknown): SessionReadResponse["branches"][number] {
  const branch = value as Record<string, unknown>;
  const result = branch.result as Record<string, unknown> | undefined;
  const parent = typeof branch.parent === "string" ? (branch.parent as BranchId) : undefined;
  const normalizedResult =
    result === undefined
      ? undefined
      : {
          id: String(result.id),
          references: Array.isArray(result.references)
            ? result.references.filter(
                (reference): reference is string => typeof reference === "string",
              )
            : [],
        };
  return {
    id: String(branch.id) as BranchId,
    ...(parent === undefined ? {} : { parent }),
    children: Array.isArray(branch.children)
      ? branch.children
          .filter((child): child is string => typeof child === "string")
          .map((child) => child as BranchId)
      : [],
    title: typeof branch.title === "string" ? branch.title : "",
    status: normalizeStatus(branch.status),
    turns: (Array.isArray(branch.turns) ? branch.turns : []).map(normalizeTurn),
    ...(Array.isArray(branch.activity)
      ? { activity: branch.activity.map(normalizeActivity).filter(isActivityItem) }
      : {}),
    ...(normalizedResult === undefined ? {} : { result: normalizedResult }),
  };
}

/** Normalizes one safe causal activity item.
 * @param value Unknown activity from the protocol transport.
 * @returns A typed activity item, or `undefined` when required data is invalid.
 */
function normalizeActivity(value: unknown): SessionActivityItem | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const item = value as Record<string, unknown>;
  const id = item.id;
  const session = item.session;
  const branch = item.branch;
  const sequence = item.sequence;
  const kind = item.kind;
  const state = item.state;
  if (
    typeof id !== "string" ||
    typeof session !== "string" ||
    typeof branch !== "string" ||
    typeof sequence !== "number" ||
    typeof item.summary !== "string" ||
    !isActivityKind(kind) ||
    !isActivityState(state)
  )
    return undefined;
  const parent = item.parent;
  const model = item.model;
  const thinkingLevel = item.thinkingLevel ?? item.thinking_level;
  const toolName = item.toolName ?? item.tool_name;
  const toolArguments = item.toolArguments ?? item.tool_arguments;
  const toolResultPreview = item.toolResultPreview ?? item.tool_result_preview;
  const durableRef = item.durableRef ?? item.durable_ref;
  return {
    id,
    session,
    branch,
    sequence,
    kind,
    state,
    summary: item.summary,
    ...(typeof parent === "string" ? { parent } : {}),
    ...(typeof model === "string" ? { model } : {}),
    ...(typeof thinkingLevel === "string" ? { thinkingLevel } : {}),
    ...(typeof toolName === "string" ? { toolName } : {}),
    ...(isObject(toolArguments) ? { toolArguments } : {}),
    ...(isObject(toolResultPreview) ? { toolResultPreview } : {}),
    ...(typeof durableRef === "string" ? { durableRef } : {}),
  };
}

/** Checks whether a value is a structured JSON object.
 * @param value Unknown protocol field.
 * @returns Whether the value can represent tool argument fields.
 */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Checks a protocol activity kind.
 * @param value Unknown kind.
 * @returns Whether the kind is supported.
 */
function isActivityKind(value: unknown): value is SessionActivityItem["kind"] {
  return [
    "HumanInput",
    "ModelInvocation",
    "AssistantOutput",
    "ToolInvocation",
    "ToolResult",
    "ChildBranch",
    "BranchResult",
  ].includes(value as string);
}

/** Checks a protocol activity state.
 * @param value Unknown state.
 * @returns Whether the state is supported.
 */
function isActivityState(value: unknown): value is SessionActivityItem["state"] {
  return value === "Completed" || value === "Running" || value === "Failed";
}

/** Narrows an optional activity item for array filtering.
 * @param value Potential activity item.
 * @returns Whether the item passed protocol validation.
 */
function isActivityItem(value: SessionActivityItem | undefined): value is SessionActivityItem {
  return value !== undefined;
}

/** Normalizes a branch status. @param value Unknown status. @returns Public status. */
function normalizeStatus(value: unknown): "Active" | "Completed" | "Abandoned" {
  const status = typeof value === "string" ? value.toLowerCase() : "active";
  return status === "completed" ? "Completed" : status === "abandoned" ? "Abandoned" : "Active";
}

/** Normalizes a wire turn. @param value Unknown turn. @returns Public turn. */
function normalizeTurn(value: unknown): SessionReadResponse["branches"][number]["turns"][number] {
  const turn = value as Record<string, unknown>;
  const agentOutput = turn.agentOutput ?? turn.agent_output;
  const toolInvocation = turn.toolInvocation ?? turn.tool_invocation;
  const toolResult = turn.toolResult ?? turn.tool_result;
  return {
    id: String(turn.id),
    ordinal: typeof turn.ordinal === "number" ? turn.ordinal : 0,
    hasHumanInput: Boolean(turn.hasHumanInput ?? turn.has_human_input),
    hasAgentOutput: Boolean(turn.hasAgentOutput ?? turn.has_agent_output),
    ...(typeof agentOutput === "string" ? { agentOutput } : {}),
    ...(typeof toolInvocation === "string" ? { toolInvocation } : {}),
    ...(typeof toolResult === "string" ? { toolResult } : {}),
  };
}

/** Projects a branch descriptor. @param branch Durable branch. @returns Compact descriptor. */
export function branchDescriptor(
  branch: SessionReadResponse["branches"][number],
): BranchChildrenPage["items"][number] {
  return {
    branchId: branch.id,
    name: branch.title,
    status: branch.status.toLowerCase() as BranchChildrenPage["items"][number]["status"],
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
}

/** Projects bounded branch activity. @param branch Durable branch. @returns Activity page. */
export function branchActivity(
  branch: SessionReadResponse["branches"][number],
): BranchActivityPage {
  const recorded = branch.activity ?? [];
  if (recorded.length > 0) {
    return {
      branchId: branch.id,
      ...(recorded[0]?.session === undefined
        ? {}
        : { sessionId: recorded[0].session as SessionId }),
      items: recorded.map((item) => ({
        id: item.id,
        ordinal: item.sequence,
        kind: activityKind(item.kind),
        activityKind: item.kind,
        state: item.state,
        summary: item.summary,
        handles:
          item.kind === "ChildBranch"
            ? [item.id]
            : item.durableRef === undefined
              ? []
              : [item.durableRef],
        ...(item.parent === undefined ? {} : { parentId: item.parent }),
        ...(item.model === undefined ? {} : { model: item.model }),
        ...(item.thinkingLevel === undefined ? {} : { thinkingLevel: item.thinkingLevel }),
        ...(item.toolName === undefined ? {} : { toolName: item.toolName }),
        ...(item.toolArguments === undefined ? {} : { toolArguments: item.toolArguments }),
        ...(item.toolResultPreview === undefined
          ? {}
          : { toolResultPreview: item.toolResultPreview }),
      })),
    };
  }
  return {
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
  };
}

/** Converts a protocol activity kind to its presentation category.
 * @param kind Durable protocol activity kind.
 * @returns Compact category used by the activity view.
 */
function activityKind(
  kind: NonNullable<SessionReadResponse["branches"][number]["activity"]>[number]["kind"],
): BranchActivityPage["items"][number]["kind"] {
  switch (kind) {
    case "HumanInput":
      return "human";
    case "ModelInvocation":
    case "AssistantOutput":
      return "model";
    case "ToolInvocation":
    case "ToolResult":
      return "tool";
    case "ChildBranch":
      return "child";
    case "BranchResult":
      return "result";
  }
}
