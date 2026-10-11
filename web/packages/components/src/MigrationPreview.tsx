/** @file Renders a migration review without owning mutation state. */
import type { ReactElement } from "react";

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
  changes: readonly string[];
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
          <li key={change}>{change}</li>
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
