/** @file Renders operation state without owning operation cancellation. */

import type { ReactElement } from "react";
import type { Operation } from "@scriberig/model";

/** Displays durable status and transient output separately. */
export function AgentOperationPanel({
  operation,
  stream,
  onCancel,
  onReviewCandidate,
}: {
  operation: Operation;
  stream?: string;
  onCancel?: () => void;
  onReviewCandidate?: (migrationId: string) => void;
}): ReactElement {
  return (
    <section aria-label={`Agent operation ${operation.operationId}`}>
      <p>Status: {operation.status}</p>
      {stream ? <output aria-label="Transient model output">{stream}</output> : null}
      {operation.result ? <p>Result: {operation.result.finalOutput}</p> : null}
      {(operation.result?.candidateChanges ?? []).map((candidate) =>
        candidate.migrationId && onReviewCandidate ? (
          <button
            key={candidate.migrationId}
            type="button"
            onClick={() => onReviewCandidate(candidate.migrationId ?? "")}
          >
            Review candidate {candidate.migrationId}
          </button>
        ) : null,
      )}
      {operation.failure ? <p role="alert">{operation.failure.message}</p> : null}
      {operation.cancellable !== false &&
      (operation.status === "queued" || operation.status === "running") &&
      onCancel ? (
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      ) : null}
    </section>
  );
}
