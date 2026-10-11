/** @file Provides the public projection host surface. */
import { MuiProjectionHost } from "@scriberig/components-mui";
import type { ProjectionHandler } from "@scriberig/components";
import type { ReactElement } from "react";
/** Hosts registered projection handlers without private fallbacks. */
/** @returns The rendered ProjectionView view or runtime value. */
export function ProjectionView({
  handlers = [],
}: {
  handlers?: readonly ProjectionHandler[];
}): ReactElement {
  return (
    <section aria-label="Projection">
      <h1>Projection</h1>
      <MuiProjectionHost projectionId="default" handlers={handlers} readonly />
    </section>
  );
}
