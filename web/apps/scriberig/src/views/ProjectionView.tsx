/** @file Provides the public projection host surface. */
import { MuiProjectionHost } from "@scriberig/components-mui";
import type { ProjectionHandler } from "@scriberig/components";
import type { ReactElement } from "react";
import { UnsupportedCapability } from "../components/UnsupportedCapability";
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
      <UnsupportedCapability capability="projection execution" />
      <MuiProjectionHost projectionId="default" handlers={handlers} readonly />
    </section>
  );
}
