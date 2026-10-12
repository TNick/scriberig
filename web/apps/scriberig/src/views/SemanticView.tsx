/** @file Provides exact semantic object inspection. */

import { MuiObjectInspector } from "@scriberig/components-mui";
import { useObject, useSelection, useScriberig, useSnapshot } from "@scriberig/react";
import { useMemo, type ReactElement } from "react";
import { SelectionSummary } from "../components/SelectionSummary";
import type { AppRuntime } from "../runtime";

/** Displays the selected semantic object at its exact snapshot. */
/** @returns The rendered semantic inspection view. */
export function SemanticView(): ReactElement {
  const runtime = useScriberig() as unknown as AppRuntime;
  const { selection, setSelection } = useSelection();
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
      {runtime.activeWorkspace ? (
        <p role="status">
          {runtime.activeWorkspace.project.path} · snapshot {runtime.activeWorkspace.snapshot}
        </p>
      ) : null}
      <ul aria-label="Semantic project symbols">
        {runtime.symbols.map((symbol) => (
          <li key={symbol.entityId}>
            <button
              type="button"
              onClick={() =>
                setSelection({
                  entityId: symbol.entityId as never,
                  snapshotId: symbol.snapshotId as never,
                })
              }
            >
              {symbol.name} · {symbol.kind} · {symbol.path}
            </button>
          </li>
        ))}
      </ul>
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
