/** @file Normalizes and projects durable session reads for browser clients. */
import type {
  BranchActivityPage,
  BranchChildrenPage,
  BranchId,
  BranchResultReference,
  SessionId,
  SessionReadResponse,
  SessionActivityItem,
  SessionExecutionChange,
  SessionPlan,
  SessionPlanNode,
  SessionPlanNodeStatus,
} from "@scriberig/protocol";
import { branchActivity, branchDescriptor } from "./session-activity-views.js";

export { branchActivity, branchDescriptor } from "./session-activity-views.js";

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
    plans: (Array.isArray(record.plans) ? record.plans : []).map(normalizePlan),
    execution: normalizeExecution(record.execution),
  };
}

/** Normalizes a durable plan without exposing transcripts or credentials.
 * @param value Unknown plan value from the transport.
 * @returns A safe plan projection.
 * @throws An error when plan normalization encounters an invalid value.
 */
function normalizePlan(value: unknown): SessionPlan {
  const plan = isObject(value) ? value : {};
  return {
    id: typeof plan.id === "string" ? plan.id : "",
    rootBranch: String(plan.rootBranch ?? plan.root_branch ?? "") as BranchId,
    nodes: (Array.isArray(plan.nodes) ? plan.nodes : []).map(normalizePlanNode),
    dependencies: (Array.isArray(plan.dependencies) ? plan.dependencies : []).flatMap((value) => {
      if (!isObject(value)) return [];
      const prerequisite = value.prerequisite;
      const dependent = value.dependent;
      if (typeof prerequisite !== "string" || typeof dependent !== "string") return [];
      const fulfilledBy = value.fulfilledBy ?? value.fulfilled_by;
      return [
        {
          prerequisite,
          dependent,
          ...(typeof fulfilledBy === "string" ? { fulfilledBy } : {}),
        },
      ];
    }),
    ...(typeof (plan.coordinator ?? plan.coordinator_branch) === "string"
      ? { coordinator: String(plan.coordinator ?? plan.coordinator_branch) as BranchId }
      : {}),
    ...(typeof (plan.coordinatorCursor ?? plan.coordinator_cursor) === "string"
      ? { coordinatorCursor: String(plan.coordinatorCursor ?? plan.coordinator_cursor) }
      : {}),
  };
}

/** Normalizes a planned work node and safe model assignment.
 * @param value Unknown plan-node value.
 * @returns A bounded node projection.
 * @throws An error when node normalization encounters an invalid value.
 */
function normalizePlanNode(value: unknown): SessionPlanNode {
  const node = isObject(value) ? value : {};
  const status = node.status;
  const validStatuses: readonly SessionPlanNodeStatus[] = [
    "Planned",
    "Ready",
    "Running",
    "Completed",
    "Failed",
    "Abandoned",
  ];
  const assignmentValue = node.assignment;
  const assignment = isObject(assignmentValue) ? assignmentValue : undefined;
  const thinkingLevel = assignment?.thinkingLevel ?? assignment?.thinking_level;
  const branch = node.branch;
  const result = node.result;
  return {
    id: typeof node.id === "string" ? node.id : "",
    parent: String(node.parent ?? ""),
    title: typeof node.title === "string" ? node.title : "",
    purpose: typeof node.purpose === "string" ? node.purpose : "",
    status: validStatuses.includes(status as SessionPlanNodeStatus)
      ? (status as SessionPlanNodeStatus)
      : "Planned",
    ...(typeof branch === "string" ? { branch: branch as BranchId } : {}),
    ...(typeof result === "string" ? { result } : {}),
    ...(typeof assignment?.profile === "string" && typeof assignment.model === "string"
      ? {
          assignment: {
            profile: assignment.profile,
            model: assignment.model,
            ...(typeof thinkingLevel === "string" ? { thinkingLevel } : {}),
          },
        }
      : {}),
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
    ...(Array.isArray(record.history)
      ? { history: record.history.map(normalizeExecutionChange).filter(isExecutionChange) }
      : {}),
  };
}

/** Normalizes one durable model/thinking selection history item.
 * @param value Unknown history item.
 * @returns A typed item, or undefined for malformed values.
 */
function normalizeExecutionChange(value: unknown): SessionExecutionChange | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const item = value as Record<string, unknown>;
  const kind = item.kind;
  if (kind !== "ModelSelectionChanged" && kind !== "ThinkingLevelChanged") return undefined;
  if (typeof item.revision !== "number" || typeof item.summary !== "string") return undefined;
  return {
    revision: item.revision,
    kind,
    summary: item.summary,
    ...(typeof item.model === "string" ? { model: item.model } : {}),
    ...(typeof item.thinking_level === "string"
      ? { thinkingLevel: item.thinking_level }
      : typeof item.thinkingLevel === "string"
        ? { thinkingLevel: item.thinkingLevel }
        : {}),
  };
}

/** Narrows normalized execution history items.
 * @param value Candidate selection history item.
 * @returns Whether the candidate was valid.
 */
function isExecutionChange(
  value: SessionExecutionChange | undefined,
): value is SessionExecutionChange {
  return value !== undefined;
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
  const toolCategory = item.toolCategory ?? item.tool_category;
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
    ...(isToolCategory(toolCategory) ? { toolCategory } : {}),
    ...(isObject(toolArguments) ? { toolArguments } : {}),
    ...(isObject(toolResultPreview) ? { toolResultPreview } : {}),
    ...(typeof durableRef === "string" ? { durableRef } : {}),
  };
}

/** Checks a trusted runtime tool category from the session protocol.
 * @param value Candidate category value.
 * @returns Whether the value is a supported category.
 */
function isToolCategory(value: unknown): value is NonNullable<SessionActivityItem["toolCategory"]> {
  return [
    "SemanticRead",
    "SemanticMutation",
    "ProjectionOperation",
    "ValidationProcess",
    "OpaqueFallback",
    "BranchProducing",
  ].includes(String(value));
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
    "ReasoningSummary",
    "RuntimeStatus",
    "ModelRequest",
    "BranchStarted",
    "BranchCompleted",
    "OperationProgress",
    "Validation",
    "ModelSelectionChanged",
    "ThinkingLevelChanged",
    "Error",
  ].includes(value as string);
}

/** Checks a protocol activity state.
 * @param value Unknown state.
 * @returns Whether the state is supported.
 */
function isActivityState(value: unknown): value is SessionActivityItem["state"] {
  return (
    value === "Completed" || value === "Running" || value === "Failed" || value === "Cancelled"
  );
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
