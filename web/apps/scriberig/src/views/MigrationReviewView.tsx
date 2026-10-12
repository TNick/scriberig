/** @file Provides migration proposal review. */
import { MuiMigrationPreview } from "@scriberig/components-mui";
import type { MigrationReviewState } from "@scriberig/editor-core";
import type { Operation } from "@scriberig/protocol";
import type { SemanticDiff } from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useEffect, useState, type ReactElement } from "react";

/** Waits for one public operation to reach a terminal lifecycle state.
 * @param client Public Scriberig client.
 * @param operationId Accepted operation identity.
 * @returns The terminal operation record.
 * @throws When the operation fails, is cancelled, or never reaches a terminal state.
 */
async function waitForTerminalOperation(
  client: ReturnType<typeof useScriberig>["client"],
  operationId: string,
): Promise<Operation> {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    const operation = await client.getOperation(operationId);
    if (operation.status === "completed") return operation;
    if (operation.status === "failed" || operation.status === "cancelled") {
      throw new Error(operation.error ?? `migration operation ${operation.status}`);
    }
    await new Promise((resolve) => window.setTimeout(resolve, 100));
  }
  throw new Error("migration operation did not reach a terminal state");
}

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
  const [actionState, setActionState] = useState<
    "idle" | "applying" | "applied" | "rejected" | "error"
  >("idle");
  const [actionMessage, setActionMessage] = useState<string>();

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
  const apply = async (): Promise<void> => {
    if (effectiveReview.baseSnapshot === undefined) {
      setActionState("error");
      setActionMessage("Cannot apply a proposal without an exact base snapshot.");
      return;
    }
    setActionState("applying");
    setActionMessage("Applying migration proposal…");
    try {
      const operation = await client.command("apply", {
        base_snapshot: effectiveReview.baseSnapshot,
        change: effectiveReview.proposalId,
      });
      await waitForTerminalOperation(client, operation.operationId);
      setActionState("applied");
      setActionMessage("Migration applied; waiting for authoritative state refresh.");
      onApply?.();
    } catch (error) {
      setActionState("error");
      setActionMessage(error instanceof Error ? error.message : "Migration application failed.");
    }
  };
  const reject = (): void => {
    setActionState("rejected");
    setActionMessage("Migration review rejected locally; canonical state was not changed.");
    onReject?.();
  };
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
        {...(actionState === "idle" || actionState === "error" ? { onApply: apply } : {})}
        {...(actionState === "idle" || actionState === "error" ? { onReject: reject } : {})}
      />
      {actionMessage ? (
        <p role={actionState === "error" ? "alert" : "status"}>{actionMessage}</p>
      ) : null}
    </section>
  );
}
