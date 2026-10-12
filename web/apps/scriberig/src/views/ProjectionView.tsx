/** @file Provides the public projection host surface. */
import { MuiObjectInspector, MuiProjectionHost } from "@scriberig/components-mui";
import type { ProjectionHandler } from "@scriberig/components";
import { useObject, useSelection, useSnapshot } from "@scriberig/react";
import { useMemo, type ReactElement } from "react";

/** Renders the built-in projection from one exact semantic selection.
 * @returns The rendered default projection state.
 */
function DefaultProjection(): ReactElement {
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

  if (!selection) return <p role="status">Select a semantic object to render.</p>;
  if (object.status === "loading") return <p role="status">Rendering projection…</p>;
  if (object.status === "error") {
    return <p role="alert">Unable to render the selected semantic object.</p>;
  }
  if (object.status !== "loaded") return <p role="status">Projection is unavailable.</p>;
  return (
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
  );
}
/** Hosts registered projection handlers without private fallbacks. */
/** @returns The rendered ProjectionView view or runtime value. */
export function ProjectionView({
  handlers = [],
}: {
  handlers?: readonly ProjectionHandler[];
}): ReactElement {
  const { selection } = useSelection();
  return (
    <section aria-label="Projection">
      <h1>Projection</h1>
      {handlers.length > 0 ? (
        <MuiProjectionHost
          projectionId="default"
          handlers={handlers}
          readonly
          {...(selection ? { selection: JSON.stringify(selection) } : {})}
        />
      ) : (
        <DefaultProjection />
      )}
    </section>
  );
}
