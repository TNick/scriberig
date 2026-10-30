/** @file Displays durable materialization failure and recovery receipts. */
import type { ProjectCandidateApplyFailure } from "@scriberig/protocol";
import type { ReactElement } from "react";

/** Presents aggregate and per-path candidate materialization state.
 * @param props Durable candidate failure receipt.
 * @returns Accessible failure and recovery details.
 */
export function ProjectCandidateFailureStatus({
  failure,
}: {
  /** Exact server receipt returned by a failed apply operation. */
  failure: ProjectCandidateApplyFailure;
}): ReactElement {
  return (
    <section aria-label="Materialization recovery status">
      <p role="alert">
        Apply failed ({failure.code}); materialization is {failure.materialization}.
      </p>
      {failure.diagnostic ? <p>{failure.diagnostic}</p> : null}
      <ul aria-label="Materialization path receipts">
        {failure.paths.map((path) => (
          <li key={path.path}>
            {path.path}: {path.state}
            {path.diagnostic ? ` — ${path.diagnostic}` : ""}
          </li>
        ))}
      </ul>
      {failure.pathsOmitted > 0 ? (
        <p role="status">
          Showing {failure.paths.length} of {failure.pathCount} path receipts;{" "}
          {failure.pathsOmitted} are omitted.
        </p>
      ) : null}
    </section>
  );
}
