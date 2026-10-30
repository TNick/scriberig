/** @file Preserves durable source materialization receipts from failed apply calls. */
import { ServerProtocolError } from "@scriberig/client";
import type { ProjectCandidateApplyFailure } from "@scriberig/protocol";

/** Identifies an apply failure with a durable filesystem recovery receipt.
 * Public members: `failure`.
 * Private members: none.
 */
export class ProjectCandidateApplyFailureError extends Error {
  /** Structured aggregate and per-path recovery state. */
  readonly failure: ProjectCandidateApplyFailure;

  /** Creates an apply error that retains its materialization receipt.
   * @param message Safe server diagnostic.
   * @param failure Durable aggregate and per-path receipt.
   */
  constructor(message: string, failure: ProjectCandidateApplyFailure) {
    super(message);
    this.name = "ProjectCandidateApplyFailureError";
    this.failure = failure;
  }
}

/** Converts a structured server error into a typed materialization failure.
 * @param error Rejected transport value from project candidate apply.
 * @returns Typed recovery information when the error includes durable receipts.
 */
export function projectCandidateApplyFailure(
  error: unknown,
): ProjectCandidateApplyFailureError | undefined {
  if (!(error instanceof ServerProtocolError)) return undefined;
  const details = error.error.details;
  if (!isRecord(details) || typeof details.candidate !== "string") return undefined;
  if (typeof details.materialization !== "string" || !Array.isArray(details.paths))
    return undefined;
  const paths = details.paths.flatMap((item) => {
    if (!isRecord(item) || typeof item.path !== "string" || typeof item.state !== "string") {
      return [];
    }
    return [
      {
        path: item.path,
        state: item.state,
        ...(typeof item.diagnostic === "string" ? { diagnostic: item.diagnostic } : {}),
      },
    ];
  });
  const failure: ProjectCandidateApplyFailure = {
    code: error.error.code,
    candidate: details.candidate,
    materialization: details.materialization,
    ...(typeof details.diagnostic === "string" ? { diagnostic: details.diagnostic } : {}),
    paths,
    pathCount: count(details.pathCount, paths.length),
    pathsOmitted: count(details.pathsOmitted, 0),
  };
  return new ProjectCandidateApplyFailureError(error.error.message, failure);
}

/** Checks whether a decoded JSON value is a plain record.
 * @param value Value to inspect.
 * @returns True when the value is a non-null non-array object.
 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Accepts a nonnegative safe integer or uses a supplied fallback.
 * @param value Untrusted count from protocol details.
 * @param fallback Count to use when the value is malformed.
 * @returns A nonnegative integer count.
 */
function count(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : fallback;
}
