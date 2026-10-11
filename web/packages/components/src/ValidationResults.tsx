/** @file Renders structured validation status and diagnostics. */
import type { ReactElement } from "react";

/** Displays validation state without collapsing diagnostics into a message. */
export function ValidationResults({
  status,
  diagnostics,
  className,
}: {
  status: "current" | "failed" | "stale" | "unknown";
  diagnostics: readonly string[];
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label="Validation results">
      <p role={status === "failed" ? "alert" : "status"}>Validation: {status}</p>
      <ul>
        {diagnostics.map((diagnostic) => (
          <li key={diagnostic}>{diagnostic}</li>
        ))}
      </ul>
    </section>
  );
}
