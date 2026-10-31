/** @file Normalizes bounded safe plan-attempt pages from the protocol. */

import type {
  SessionPlanAttemptSummary,
  SessionPlanAttemptsResponse,
  SessionPlanNodeStatus,
} from "@scriberig/protocol";
import { normalizeEvidenceReference } from "./session-plan-semantic-views.js";

const statuses: readonly SessionPlanNodeStatus[] = [
  "Planned",
  "Ready",
  "Running",
  "Completed",
  "Failed",
  "Abandoned",
];

/** Normalizes one bounded safe attempt-history page.
 * @param value Untrusted response value from the transport.
 * @returns A safe normalized plan-attempt page.
 */
export function normalizePlanAttempts(value: unknown): SessionPlanAttemptsResponse {
  const record = asRecord(value);
  const attempts = Array.isArray(record.attempts) ? record.attempts.map(normalizeAttempt) : [];
  return {
    plan: text(record.plan),
    node: text(record.node),
    total: count(record.total),
    offset: count(record.offset),
    attempts,
    ...(record.next_offset === undefined || record.next_offset === null
      ? {}
      : { nextOffset: count(record.next_offset) }),
  };
}

/** Normalizes one transcript-free worker attempt row.
 * @param value Untrusted attempt value from the transport.
 * @returns A safe normalized attempt.
 */
function normalizeAttempt(value: unknown): SessionPlanAttemptSummary {
  const record = asRecord(value);
  const status = text(record.status) as SessionPlanNodeStatus;
  const failure =
    record.failure === undefined || record.failure === null
      ? undefined
      : normalizeEvidenceReference(record.failure);
  return {
    branch: text(record.branch) as SessionPlanAttemptSummary["branch"],
    claimGeneration: count(record.claim_generation),
    ...(record.claim == null ? {} : { claim: text(record.claim) }),
    status: statuses.includes(status) ? status : "Planned",
    ...(record.result == null ? {} : { result: text(record.result) }),
    ...(failure === undefined ? {} : { failure }),
  };
}

/** Narrows an unknown JSON value to a record.
 * @param value Candidate JSON value.
 * @returns The record or an empty object.
 */
function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

/** Reads a string-valued field safely.
 * @param value Candidate field value.
 * @returns The string or an empty value.
 */
function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** Reads a nonnegative integral count safely.
 * @param value Candidate count field.
 * @returns The safe integer count or zero.
 */
function count(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : 0;
}
