/** @file Provides exact semantic object inspection. */

import { MuiObjectInspector } from "@scriberig/components-mui";
import { useObject, useSelection, useSnapshot } from "@scriberig/react";
import { useMemo, type ReactElement } from "react";
import { SelectionSummary } from "../components/SelectionSummary";

/** Displays the selected semantic object at its exact snapshot. */
/** @returns The rendered semantic inspection view. */
export function SemanticView(): ReactElement {
  const { selection } = useSelection();
  const request = useMemo(
    () =>
      selection
        ? {
            entityId: selection.entityId,
            snapshot: { kind: "exact" as const, snapshotId: selection.snapshotId },
            detail: "deep" as const,
          }
        : undefined,
    [selection],
  );
  const object = useObject(request);
  const snapshot = useSnapshot(selection?.snapshotId);

  return (
    <section aria-label="Semantic">
      <h1>Semantic</h1>
      <SelectionSummary selection={selection} />
      {object.status === "loading" ? <p role="status">Loading semantic object…</p> : null}
      {object.status === "error" ? (
        <p role="alert">Unable to inspect the selected object.</p>
      ) : null}
      {object.status === "loaded" ? (
        <MuiObjectInspector
          value={{
            summary: object.value.summary,
            ...(object.value.text === undefined ? {} : { description: object.value.text }),
            state:
              snapshot.kind === "candidate"
                ? "historical"
                : snapshot.kind === "current"
                  ? "current"
                  : snapshot.kind,
          }}
        />
      ) : null}
    </section>
  );
}
