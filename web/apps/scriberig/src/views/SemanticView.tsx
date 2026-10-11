/** @file Provides generic semantic inspection fallback. */
import { useScriberig } from "@scriberig/react";
import { SelectionSummary } from "../components/SelectionSummary";
import type { ReactElement } from "react";
/** Displays the shared semantic selection. */
/** @returns The rendered SemanticView view or runtime value. */
export function SemanticView(): ReactElement {
  const { selection } = useScriberig();
  return (
    <section aria-label="Semantic">
      <h1>Semantic</h1>
      <SelectionSummary selection={selection.get()} />
    </section>
  );
}
