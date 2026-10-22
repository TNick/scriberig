/** @file Normalizes bounded plan gate and evidence detail pages. */

import type { SessionPlanDetailsResponse } from "@scriberig/protocol";
import { normalizeEvidence, normalizeGate } from "./session-plan-semantic-views.js";

/** Normalizes bounded plan details from an untrusted protocol response.
 * @param value Untrusted wire response.
 * @returns Safe gate and evidence pages tied to the returned revision.
 */
export function normalizePlanDetails(value: unknown): SessionPlanDetailsResponse {
  const record = asRecord(value);
  return {
    plan: text(record.plan),
    revision: count(record.revision),
    ...(record.snapshot == null ? {} : { snapshot: text(record.snapshot) }),
    gateTotal: count(record.gate_total),
    gateOffset: count(record.gate_offset),
    gates: asArray(record.gates).flatMap(normalizeGate),
    ...(record.gate_next_offset == null ? {} : { gateNextOffset: count(record.gate_next_offset) }),
    evidenceTotal: count(record.evidence_total),
    evidenceOffset: count(record.evidence_offset),
    evidence: asArray(record.evidence).flatMap(normalizeEvidence),
    ...(record.evidence_next_offset == null
      ? {}
      : { evidenceNextOffset: count(record.evidence_next_offset) }),
  };
}

/** Narrows an unknown JSON value to a record.
 * @param value Candidate JSON value.
 * @returns The record or an empty object.
 */
function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

/** Narrows an unknown JSON value to an array.
 * @param value Candidate JSON value.
 * @returns The array or an empty list.
 */
function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
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
