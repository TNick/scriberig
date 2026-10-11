/** @file Frames registered domain projections with Material UI state. */
import type { ReactElement } from "react";
import { MuiPanel, MuiStatus } from "./mui-primitives";
import type { ProjectionHandler } from "@scriberig/components";
/** Renders the MuiProjectionHost Material UI component. */
export function MuiProjectionHost({
  projectionId,
  handlers,
  selection,
  readonly,
  stale,
  conflict,
  className,
}: {
  projectionId: string;
  handlers: readonly ProjectionHandler[];
  selection?: string;
  readonly?: boolean;
  stale?: boolean;
  conflict?: boolean;
  className?: string;
}): ReactElement {
  const h = handlers.find((x) => x.id === projectionId);
  return (
    <MuiPanel title="Projection" className={className}>
      {readonly ? <MuiStatus>Readonly</MuiStatus> : null}
      {stale ? <MuiStatus severity="warning">Stale projection</MuiStatus> : null}
      {conflict ? <MuiStatus severity="error">Projection conflict</MuiStatus> : null}
      {h ? (
        h.render(selection === undefined ? {} : { selection })
      ) : (
        <MuiStatus severity="warning">Unsupported projection: {projectionId}</MuiStatus>
      )}
    </MuiPanel>
  );
}
