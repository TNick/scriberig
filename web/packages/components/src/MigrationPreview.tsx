/** @file Renders a migration review without owning mutation state. */
import type { ReactElement } from "react";
import type { MigrationChange } from "./component-types";

/**
 * Formats a structured migration change for a neutral presentation.
 * @param change Structured migration change.
 * @returns Human-readable change text.
 */
function formatChange(change: MigrationChange): string {
  if (typeof change === "string") return change;
  return `${change.kind}: ${change.entityId}`;
}

/** Presents migration status and caller-controlled review actions. */
export function MigrationPreview({
  baseSnapshot,
  changes,
  validation,
  state,
  onApply,
  onReject,
  className,
}: {
  baseSnapshot: string;
  changes: readonly MigrationChange[];
  validation: "current" | "failed" | "unknown";
  state?: "ready" | "stale" | "conflict";
  onApply?: () => void;
  onReject?: () => void;
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label="Migration preview">
      <p>Base: {baseSnapshot}</p>
      <p role={validation === "failed" ? "alert" : "status"}>Validation: {validation}</p>
      {state ? <p role="status">State: {state}</p> : null}
      <ul>
        {changes.map((change) => (
          <li key={formatChange(change)}>{formatChange(change)}</li>
        ))}
      </ul>
      {onApply ? (
        <button
          type="button"
          disabled={validation === "failed" || state === "stale" || state === "conflict"}
          onClick={onApply}
        >
          Apply
        </button>
      ) : null}
      {onReject ? (
        <button type="button" onClick={onReject}>
          Reject
        </button>
      ) : null}
    </section>
  );
}
