/** @file Provides semantic history and diff navigation. */
import { MuiHistoryView, MuiSemanticDiffView } from "@scriberig/components-mui";
import type { DiffEntry } from "@scriberig/components";
import { useSelection, useScriberig } from "@scriberig/react";
import { useEffect, useState, type ReactElement } from "react";
/** Renders caller-provided semantic history entries. */
/** @returns The rendered HistoryView view or runtime value. */
export function HistoryView({
  entries: suppliedEntries,
}: {
  entries?: readonly DiffEntry[];
}): ReactElement {
  const { client } = useScriberig();
  const { selection } = useSelection();
  const [loadedEntries, setLoadedEntries] = useState<readonly DiffEntry[]>([]);

  useEffect(() => {
    if (!selection || suppliedEntries !== undefined) return;
    const controller = new AbortController();
    void client
      .history(
        { entityId: selection.entityId, to: { kind: "exact", snapshotId: selection.snapshotId } },
        controller.signal,
      )
      .then((response) => {
        if (!Array.isArray(response)) return;
        setLoadedEntries(
          response.map((summary) => ({
            id: `${summary.ref.entityId}:${summary.ref.snapshotId}`,
            kind: "snapshot",
            summary: summary.label ?? summary.ref.entityId,
          })),
        );
      })
      .catch(() => {
        if (!controller.signal.aborted) setLoadedEntries([]);
      });
    return () => controller.abort();
  }, [client, selection, suppliedEntries]);

  const entries = suppliedEntries ?? loadedEntries;
  return (
    <section aria-label="History">
      <h1>History / Diff</h1>
      {entries.length === 0 ? <p role="status">No history entries loaded.</p> : null}
      <MuiHistoryView entries={entries} />
      <MuiSemanticDiffView entries={entries} />
    </section>
  );
}
