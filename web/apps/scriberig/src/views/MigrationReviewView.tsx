/** @file Provides migration proposal review. */
import { MuiMigrationPreview } from "@scriberig/components-mui";
import type { MigrationReviewState } from "@scriberig/editor-core";
import type { ReactElement } from "react";

/** Renders a caller-owned migration proposal without fabricating review state. */
export function MigrationReviewView({
  review,
  changes = [],
  validation = "unknown",
  onApply,
  onReject,
}: {
  review?: MigrationReviewState;
  changes?: readonly string[];
  validation?: "current" | "failed" | "unknown";
  onApply?: () => void;
  onReject?: () => void;
}): ReactElement {
  if (!review) {
    return (
      <section aria-label="Migration review">
        <h1>Migration Review</h1>
        <p role="status">No migration proposal selected.</p>
      </section>
    );
  }

  const state = review.conflict ? "conflict" : review.stale ? "stale" : "ready";
  return (
    <section aria-label="Migration review">
      <h1>Migration Review</h1>
      <MuiMigrationPreview
        baseSnapshot={review.baseSnapshot ?? "pending"}
        {...(review.candidateSnapshot === undefined
          ? {}
          : { candidateSnapshot: review.candidateSnapshot })}
        changes={changes}
        validation={validation}
        state={state}
        {...(onApply === undefined ? {} : { onApply })}
        {...(onReject === undefined ? {} : { onReject })}
      />
    </section>
  );
}
