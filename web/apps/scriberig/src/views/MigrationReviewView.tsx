/** @file Provides migration proposal review. */
import { MuiMigrationPreview } from "@scriberig/components-mui";
import type { MigrationReviewState } from "@scriberig/editor-core";
import type { SemanticDiff } from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useEffect, useState, type ReactElement } from "react";

/** Renders a caller-owned migration proposal without fabricating review state. */
export function MigrationReviewView({
  migrationId,
  review,
  changes,
  validation = "unknown",
  onApply,
  onReject,
}: {
  migrationId?: string;
  review?: MigrationReviewState;
  changes?: readonly SemanticDiff[];
  validation?: "current" | "failed" | "unknown";
  onApply?: () => void;
  onReject?: () => void;
}): ReactElement {
  const { client } = useScriberig();
  const [loadedReview, setLoadedReview] = useState<MigrationReviewState>();
  const [loadedChanges, setLoadedChanges] = useState<readonly SemanticDiff[]>([]);

  useEffect(() => {
    if (review !== undefined || migrationId === undefined) return;
    const controller = new AbortController();
    void client
      .migrationDetail(migrationId, controller.signal)
      .then((detail) => {
        setLoadedReview({
          proposalId: detail.migrationId,
          baseSnapshot: detail.base,
          candidateSnapshot: detail.result,
          stale: false,
          conflict: false,
        });
        setLoadedChanges(detail.changes);
      })
      .catch(() => {
        if (!controller.signal.aborted) {
          setLoadedReview(undefined);
          setLoadedChanges([]);
        }
      });
    return () => controller.abort();
  }, [client, migrationId, review]);

  const effectiveReview = review ?? loadedReview;
  const effectiveChanges = changes ?? loadedChanges;
  if (!effectiveReview) {
    return (
      <section aria-label="Migration review">
        <h1>Migration Review</h1>
        <p role="status">No migration proposal selected.</p>
      </section>
    );
  }

  const state = effectiveReview.conflict ? "conflict" : effectiveReview.stale ? "stale" : "ready";
  return (
    <section aria-label="Migration review">
      <h1>Migration Review</h1>
      <MuiMigrationPreview
        baseSnapshot={effectiveReview.baseSnapshot ?? "pending"}
        {...(effectiveReview.candidateSnapshot === undefined
          ? {}
          : { candidateSnapshot: effectiveReview.candidateSnapshot })}
        changes={effectiveChanges}
        validation={validation}
        state={state}
        {...(onApply === undefined ? {} : { onApply })}
        {...(onReject === undefined ? {} : { onReject })}
      />
    </section>
  );
}
