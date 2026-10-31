/** @file Normalizes bounded semantic plan attempt history pages. */
import type {
  SessionPlanAttempt as SessionPlanAttemptRecord,
  SessionPlanAttemptHistoryPage,
  SessionPlanNodeStatus,
} from "@scriberig/protocol";
import { normalizeEvidenceReference } from "./session-plan-semantic-views";

/** Normalizes a bounded page of exact-snapshot attempt records.
 * @param value Unknown response value from the transport.
 * @returns A safe attempt history page.
 */
export function normalizePlanAttemptHistory(value: unknown): SessionPlanAttemptHistoryPage {
  const page = isObject(value) ? value : {};
  return {
    offset: typeof page.offset === "number" ? page.offset : 0,
    attempts: (Array.isArray(page.attempts) ? page.attempts : []).flatMap((item) => {
      const attempt = normalizeAttempt(item);
      return attempt === undefined ? [] : [attempt];
    }),
  };
}

/** Normalizes one exact worker attempt.
 * @param value Unknown semantic attempt record.
 * @returns A valid attempt or `undefined` when required identities are absent.
 */
function normalizeAttempt(value: unknown): SessionPlanAttemptRecord | undefined {
  if (!isObject(value)) return undefined;
  const id = value.id;
  const exactRevision = value.exactRevision ?? value.exact_revision;
  const node = value.node;
  const branch = value.branch;
  if (
    typeof id !== "string" ||
    typeof exactRevision !== "string" ||
    typeof node !== "string" ||
    typeof branch !== "string"
  ) {
    return undefined;
  }
  const generation = value.claimGeneration ?? value.claim_generation;
  const claim = value.claim;
  const result = value.result;
  const failure = normalizeEvidenceReference(value.failure);
  return {
    id,
    exactRevision,
    node,
    branch: branch as SessionPlanAttemptRecord["branch"],
    claimGeneration: typeof generation === "number" ? generation : 0,
    ...(typeof claim === "string" ? { claim } : {}),
    status: normalizeStatus(value.status),
    ...(typeof result === "string" ? { result } : {}),
    ...(failure === undefined ? {} : { failure }),
  };
}

/** Restricts attempt status to the shared node lifecycle vocabulary.
 * @param value Unknown attempt status.
 * @returns A safe node status.
 */
function normalizeStatus(value: unknown): SessionPlanNodeStatus {
  const statuses: readonly SessionPlanNodeStatus[] = [
    "Planned",
    "Ready",
    "Running",
    "Completed",
    "Failed",
    "Abandoned",
  ];
  return statuses.includes(value as SessionPlanNodeStatus)
    ? (value as SessionPlanNodeStatus)
    : "Failed";
}

/** Checks whether a value is a plain record-like object.
 * @param value Value to inspect.
 * @returns Whether property access is safe.
 */
function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
