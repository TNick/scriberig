/** @file Normalizes durable plan state from the session protocol. */

import type {
  BranchId,
  SessionContinuationAnchor,
  SessionModelAssignment,
  SessionPlan,
  SessionPlanDependency,
  SessionPlanNode,
  SessionPlanNodeStatus,
  SessionPlanRevision,
  SessionPlanStatus,
  SessionWorkerClaim,
} from "@scriberig/protocol";

/** Normalizes one durable plan without exposing transcripts or credentials.
 * @param value Unknown plan value from the transport.
 * @returns A safe plan projection.
 */
export function normalizeSessionPlan(value: unknown): SessionPlan {
  const plan = isObject(value) ? value : {};
  const anchor = normalizeAnchor(plan.anchor);
  const coordinatorClaim = plan.coordinator_claim ?? plan.coordinatorClaim;
  return {
    id: typeof plan.id === "string" ? plan.id : "",
    revision: typeof plan.revision === "number" ? plan.revision : 0,
    goal: typeof plan.goal === "string" ? plan.goal : "",
    status: normalizePlanStatus(plan.status),
    ...(anchor === undefined ? {} : { anchor }),
    rootBranch: String(plan.rootBranch ?? plan.root_branch ?? "") as BranchId,
    nodes: (Array.isArray(plan.nodes) ? plan.nodes : []).map(normalizePlanNode),
    dependencies: normalizeDependencies(plan.dependencies),
    ...(typeof (plan.coordinator ?? plan.coordinator_branch) === "string"
      ? { coordinator: String(plan.coordinator ?? plan.coordinator_branch) as BranchId }
      : {}),
    ...(typeof (plan.coordinatorCursor ?? plan.coordinator_cursor) === "string"
      ? { coordinatorCursor: String(plan.coordinatorCursor ?? plan.coordinator_cursor) }
      : {}),
    ...(isObject(coordinatorClaim)
      ? { coordinatorClaim: normalizeCoordinatorClaim(coordinatorClaim) }
      : {}),
    history: (Array.isArray(plan.history) ? plan.history : []).map(normalizePlanRevision),
  };
}

/** Normalizes one plan lifecycle value.
 * @param value Unknown wire status.
 * @returns A safe lifecycle status.
 */
function normalizePlanStatus(value: unknown): SessionPlanStatus {
  const statuses: readonly SessionPlanStatus[] = [
    "Draft",
    "Planned",
    "Running",
    "Paused",
    "Completed",
    "Failed",
    "Cancelled",
  ];
  return statuses.includes(value as SessionPlanStatus) ? (value as SessionPlanStatus) : "Draft";
}

/** Normalizes one exact continuation anchor.
 * @param value Unknown wire anchor.
 * @returns Stable anchor or `undefined` when the input is malformed.
 */
function normalizeAnchor(value: unknown): SessionContinuationAnchor | undefined {
  if (!isObject(value)) return undefined;
  for (const key of ["BranchHead", "AssistantOutput", "HumanInput", "BranchResult"] as const) {
    const payload = value[key];
    if (!isObject(payload) || typeof payload.branch !== "string") continue;
    if (key === "BranchHead") return { BranchHead: { branch: payload.branch as BranchId } };
    if (key === "AssistantOutput" || key === "HumanInput") {
      if (typeof payload.turn === "string") {
        if (key === "AssistantOutput") {
          return {
            AssistantOutput: { branch: payload.branch as BranchId, turn: payload.turn },
          };
        }
        return { HumanInput: { branch: payload.branch as BranchId, turn: payload.turn } };
      }
    }
    if (key === "BranchResult" && typeof payload.result === "string") {
      return {
        BranchResult: { branch: payload.branch as BranchId, result: payload.result },
      };
    }
  }
  return undefined;
}

/** Normalizes plan dependencies while dropping malformed edges.
 * @param value Unknown wire dependency list.
 * @returns Valid dependency edges.
 */
function normalizeDependencies(value: unknown): SessionPlanDependency[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item) => {
    if (!isObject(item)) return [];
    const prerequisite = item.prerequisite;
    const dependent = item.dependent;
    if (typeof prerequisite !== "string" || typeof dependent !== "string") return [];
    const fulfilledBy = item.fulfilledBy ?? item.fulfilled_by;
    return [
      {
        prerequisite,
        dependent,
        ...(typeof fulfilledBy === "string" ? { fulfilledBy } : {}),
      },
    ];
  });
}

/** Normalizes a safe plan coordinator claim.
 * @param value Unknown wire claim.
 * @returns A bounded coordinator claim.
 */
function normalizeCoordinatorClaim(
  value: Readonly<Record<string, unknown>>,
): NonNullable<SessionPlan["coordinatorClaim"]> {
  return {
    id: String(value.id ?? ""),
    ...(typeof value.operation === "string" ? { operation: value.operation } : {}),
    owner: String(value.owner ?? ""),
    generation: typeof value.generation === "number" ? value.generation : 0,
  };
}

/** Normalizes one immutable plan revision.
 * @param value Unknown wire revision.
 * @returns A safe immutable revision.
 */
function normalizePlanRevision(value: unknown): SessionPlanRevision {
  const revision = isObject(value) ? value : {};
  return {
    revision: typeof revision.revision === "number" ? revision.revision : 0,
    goal: typeof revision.goal === "string" ? revision.goal : "",
    status: normalizePlanStatus(revision.status),
    nodes: (Array.isArray(revision.nodes) ? revision.nodes : []).map(normalizePlanNode),
    dependencies: normalizeDependencies(revision.dependencies),
    ...(typeof revision.reason === "string" ? { reason: revision.reason } : {}),
  };
}

/** Normalizes one planned work node and its safe worker assignment.
 * @param value Unknown wire node.
 * @returns A bounded plan node.
 */
function normalizePlanNode(value: unknown): SessionPlanNode {
  const node = isObject(value) ? value : {};
  const statuses: readonly SessionPlanNodeStatus[] = [
    "Planned",
    "Ready",
    "Running",
    "Completed",
    "Failed",
    "Abandoned",
  ];
  const assignment = normalizeAssignment(node.assignment);
  const claimValue = node.claim;
  const claim = isObject(claimValue) ? normalizeWorkerClaim(claimValue) : undefined;
  const expectedValue = node.expectedResult ?? node.expected_result;
  const expected = isObject(expectedValue) ? expectedValue : undefined;
  const semanticScope = node.semanticScope ?? node.semantic_scope;
  const branch = node.branch;
  const result = node.result;
  const supersededBy = node.supersededBy ?? node.superseded_by;
  return {
    id: typeof node.id === "string" ? node.id : "",
    parent: String(node.parent ?? ""),
    title: typeof node.title === "string" ? node.title : "",
    purpose: typeof node.purpose === "string" ? node.purpose : "",
    status: statuses.includes(node.status as SessionPlanNodeStatus)
      ? (node.status as SessionPlanNodeStatus)
      : "Planned",
    ...(typeof branch === "string" ? { branch: branch as BranchId } : {}),
    ...(typeof result === "string" ? { result } : {}),
    ...(typeof supersededBy === "string" ? { supersededBy } : {}),
    ...(expected === undefined
      ? {}
      : {
          expectedResult: {
            kind: String(expected.kind ?? ""),
            requiresHandle: Boolean(expected.requiresHandle ?? expected.requires_handle),
          },
        }),
    semanticScope: Array.isArray(semanticScope)
      ? semanticScope.filter((item): item is string => typeof item === "string")
      : [],
    ...(assignment === undefined ? {} : { assignment }),
    ...(claim === undefined ? {} : { claim }),
  };
}

/** Normalizes a safe model assignment.
 * @param value Unknown wire assignment.
 * @returns A model assignment or `undefined` when required fields are absent.
 */
function normalizeAssignment(value: unknown): SessionModelAssignment | undefined {
  if (!isObject(value) || typeof value.profile !== "string" || typeof value.model !== "string") {
    return undefined;
  }
  const thinkingLevel = value.thinkingLevel ?? value.thinking_level;
  return {
    profile: value.profile,
    model: value.model,
    ...(typeof thinkingLevel === "string" ? { thinkingLevel } : {}),
  };
}

/** Normalizes an active worker claim.
 * @param value Unknown wire claim.
 * @returns A bounded worker claim.
 */
function normalizeWorkerClaim(value: Readonly<Record<string, unknown>>): SessionWorkerClaim {
  const assignment = normalizeAssignment(value.assignment);
  return {
    id: String(value.id ?? ""),
    node: String(value.node ?? ""),
    branch: String(value.branch ?? "") as BranchId,
    owner: String(value.owner ?? ""),
    generation: typeof value.generation === "number" ? value.generation : 0,
    ...(assignment === undefined ? {} : { assignment }),
  };
}

/** Narrows a JSON object that can be read as a wire record.
 * @param value Candidate wire value.
 * @returns Whether the value is a non-array object.
 */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
