/** @file Normalizes compact server plan graphs without deriving plan semantics. */

import type {
  BranchId,
  SessionModelAssignment,
  SessionPlanGraphView,
  SessionPlanNodeStatus,
  SessionPlanStatusCounts,
} from "@scriberig/protocol";
import {
  normalizeEvidenceReference,
  normalizeGateCounts,
  normalizeStatusCounts,
} from "./session-plan-semantic-views";

const nodeStatuses: readonly SessionPlanNodeStatus[] = [
  "Planned",
  "Ready",
  "Running",
  "Completed",
  "Failed",
  "Abandoned",
];

const planStatuses: readonly SessionPlanGraphView["plan"]["status"][] = [
  "Draft",
  "Planned",
  "Running",
  "Paused",
  "Completed",
  "Failed",
  "Cancelled",
];

/** Normalizes one compact plan graph received through the server protocol.
 *
 * The server supplies all effective statuses, dependency blockers, gate counts,
 * evidence counts, and retry capabilities. This function only adapts wire names
 * and bounds malformed values; it does not reproduce those calculations.
 *
 * @param value Unknown response from the transport.
 * @returns A safe compact plan graph for TypeScript consumers.
 */
export function normalizePlanGraphView(value: unknown): SessionPlanGraphView {
  const graph = record(value);
  const plan = record(graph.plan);
  const snapshot = optionalString(plan.snapshot);
  const coordinator = optionalString(plan.coordinator);
  const coordinatorCursor = optionalString(plan.coordinatorCursor ?? plan.coordinator_cursor);
  const status = plan.status;
  return {
    plan: {
      id: stringValue(plan.id),
      revision: stringValue(plan.revision),
      ...(snapshot === undefined ? {} : { snapshot }),
      mutationRevision: count(plan.mutationRevision ?? plan.mutation_revision),
      goal: stringValue(plan.goal),
      status: planStatuses.includes(status as SessionPlanGraphView["plan"]["status"])
        ? (status as SessionPlanGraphView["plan"]["status"])
        : "Draft",
      ...(coordinator === undefined ? {} : { coordinator }),
      ...(coordinatorCursor === undefined ? {} : { coordinatorCursor }),
      rootNodes: stringList(plan.rootNodes ?? plan.root_nodes),
      nodeCount: count(plan.nodeCount ?? plan.node_count),
      statusCounts: normalizePlanCounts(plan.statusCounts ?? plan.status_counts),
    },
    nodes: (Array.isArray(graph.nodes) ? graph.nodes : []).map(normalizeNode),
  };
}

/** Normalizes one compact semantic node while retaining server calculations.
 * @param value Unknown node projection.
 * @returns A bounded node projection.
 */
function normalizeNode(value: unknown): SessionPlanGraphView["nodes"][number] {
  const node = record(value);
  const direct = node.directStatus ?? node.direct_status;
  const effective = node.effectiveStatus ?? node.effective_status;
  const assignment = normalizeAssignment(node.assignment);
  const lastFailure = normalizeEvidenceReference(node.lastFailure ?? node.last_failure);
  const snapshot = optionalString(node.snapshot);
  const parent = optionalString(node.parent);
  const summaryId = optionalString(node.summaryId ?? node.summary_id);
  const summaryPreview = optionalString(node.summaryPreview ?? node.summary_preview);
  const branch = optionalString(node.branch);
  const result = optionalString(node.result);
  return {
    id: stringValue(node.id),
    revision: stringValue(node.revision),
    ...(snapshot === undefined ? {} : { snapshot }),
    plan: stringValue(node.plan),
    ...(parent === undefined ? {} : { parent }),
    title: stringValue(node.title),
    purpose: stringValue(node.purpose),
    directStatus: normalizeNodeStatus(direct),
    effectiveStatus: normalizeNodeStatus(effective),
    containsIgnored: booleanValue(node.containsIgnored ?? node.contains_ignored),
    containsAbandoned: booleanValue(node.containsAbandoned ?? node.contains_abandoned),
    ...(summaryId === undefined ? {} : { summaryId }),
    ...(summaryPreview === undefined ? {} : { summaryPreview }),
    ...(branch === undefined ? {} : { branch: branch as BranchId }),
    ...(result === undefined ? {} : { result }),
    ...(assignment === undefined ? {} : { assignment }),
    dependencyBlockers: stringList(node.dependencyBlockers ?? node.dependency_blockers),
    entryGates: normalizeGateCounts(node.entryGates ?? node.entry_gates) ?? emptyGateCounts(),
    completionGates:
      normalizeGateCounts(node.completionGates ?? node.completion_gates) ?? emptyGateCounts(),
    evidenceCount: count(node.evidenceCount ?? node.evidence_count),
    canRetry: booleanValue(node.canRetry ?? node.can_retry),
    ...(lastFailure === undefined ? {} : { lastFailure }),
  };
}

/** Normalizes a safe upstream provider/model assignment.
 * @param value Unknown assignment value.
 * @returns A valid assignment or `undefined` when required identities are absent.
 */
function normalizeAssignment(value: unknown): SessionModelAssignment | undefined {
  const assignment = record(value);
  const providerId = optionalString(assignment.providerId ?? assignment.provider_id);
  const modelId = optionalString(assignment.modelId ?? assignment.model_id);
  if (providerId === undefined || modelId === undefined) return undefined;
  const thinkingLevel = optionalString(assignment.thinkingLevel ?? assignment.thinking_level);
  return {
    providerId,
    modelId,
    ...(thinkingLevel === undefined ? {} : { thinkingLevel }),
  };
}

/** Normalizes a node lifecycle value from the protocol.
 * @param value Unknown status.
 * @returns A valid status, defaulting malformed values to Planned.
 */
function normalizeNodeStatus(value: unknown): SessionPlanNodeStatus {
  return nodeStatuses.includes(value as SessionPlanNodeStatus)
    ? (value as SessionPlanNodeStatus)
    : "Planned";
}

/** Normalizes plan counters without deriving them from node rows.
 * @param value Unknown counter record.
 * @returns Bounded shared plan counters.
 */
function normalizePlanCounts(value: unknown): SessionPlanStatusCounts {
  return normalizeStatusCounts(value);
}

/** Creates a zero-valued gate summary for a malformed gate record.
 * @returns Empty shared gate counts.
 */
function emptyGateCounts() {
  return { pending: 0, satisfied: 0, failed: 0, waived: 0 };
}

/** Reads a JSON record without trusting its runtime shape.
 * @param value Unknown JSON value.
 * @returns The record or an empty record.
 */
function record(value: unknown): Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Readonly<Record<string, unknown>>)
    : {};
}

/** Reads an optional string field.
 * @param value Unknown field value.
 * @returns The non-empty string or `undefined`.
 */
function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

/** Reads a string field with an empty fallback.
 * @param value Unknown field value.
 * @returns The string or an empty string.
 */
function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** Reads only string entries from a wire list.
 * @param value Unknown list value.
 * @returns The string entries in source order.
 */
function stringList(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

/** Reads a finite nonnegative integer counter.
 * @param value Unknown counter value.
 * @returns A bounded count.
 */
function count(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
}

/** Reads a boolean field without truthiness coercion.
 * @param value Unknown boolean value.
 * @returns The boolean or `false`.
 */
function booleanValue(value: unknown): boolean {
  return typeof value === "boolean" ? value : false;
}
