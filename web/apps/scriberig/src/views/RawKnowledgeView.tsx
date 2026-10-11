/** @file Provides raw semantic knowledge browsing. */
import { useScriberig } from "@scriberig/react";
import { MuiObjectTree } from "@scriberig/components-mui";
import type { ReactElement } from "react";
/** Renders the shared semantic tree when available. */
/** @returns The rendered RawKnowledgeView view or runtime value. */
export function RawKnowledgeView(): ReactElement {
  const { tree } = useScriberig();
  return (
    <section aria-label="Raw Knowledge">
      <h1>Raw Knowledge</h1>
      {tree ? <MuiObjectTree tree={tree} rootId="root" /> : <p>No knowledge tree is connected.</p>}
    </section>
  );
}
