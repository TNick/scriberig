/** @file Provides semantic history and diff navigation. */
import { MuiHistoryView, MuiSemanticDiffView } from "@scriberig/components-mui";
import type { DiffEntry } from "@scriberig/components";
import type { ReactElement } from "react";
import { UnsupportedCapability } from "../components/UnsupportedCapability";
/** Renders caller-provided semantic history entries. */
/** @returns The rendered HistoryView view or runtime value. */
export function HistoryView({ entries = [] }: { entries?: readonly DiffEntry[] }): ReactElement {
  return (
    <section aria-label="History">
      <h1>History / Diff</h1>
      <UnsupportedCapability capability="history and semantic diff" />
      <MuiHistoryView entries={entries} />
      <MuiSemanticDiffView entries={entries} />
    </section>
  );
}
