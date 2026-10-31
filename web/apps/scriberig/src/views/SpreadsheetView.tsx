/** @file Composes bounded spreadsheet navigation and candidate editing for the reference app. */
import { useEffect, useMemo, useState, type ReactElement } from "react";
import type { ScriberigClient } from "@scriberig/client";
import type { SpreadsheetCellView, SpreadsheetRange } from "@scriberig/protocol";
import type { SpreadsheetCellValue } from "@scriberig/protocol";
import type { ArtifactEditCapability } from "@scriberig/protocol";
import {
  ProjectCandidateReview,
  SpreadsheetCellEditor,
  SpreadsheetPageView,
} from "@scriberig/components";
import { ProjectCandidateController, SpreadsheetPageController } from "@scriberig/model";

/** Formats a typed value as a bounded review summary.
 * @param value Semantic spreadsheet value.
 * @returns Authored value text limited to one short line.
 */
function reviewValue(value: SpreadsheetCellValue): string {
  const text =
    value.kind === "blank"
      ? "(blank)"
      : value.kind === "formula"
        ? value.value.expression
        : String(value.value);
  return text.length > 160 ? text.slice(0, 157) + "…" : text;
}

/** Renders the selected worksheet through the shared bounded client API.
 * @param props Client, exact identities, and semantic selection callback.
 * @returns Bounded worksheet page and selected-cell candidate editor.
 */
export function SpreadsheetView({
  client,
  project,
  snapshot,
  worksheet,
  onSelectCell,
  onSelectRange,
  onCandidateApplied,
}: {
  /** Shared semantic client. */
  client: ScriberigClient;
  /** Active project identity. */
  project: string;
  /** Snapshot attached to the selected worksheet. */
  snapshot: string;
  /** Selected semantic worksheet identity. */
  worksheet: string;
  /** Publishes selected cell handle and exact worksheet anchor. */
  onSelectCell: (cell: SpreadsheetCellView, snapshot: string) => void;
  /** Publishes a typed rectangular anchor with its exact snapshot. */
  onSelectRange: (range: SpreadsheetRange, snapshot: string) => void;
  /** Publishes the successor snapshot after shared candidate application. */
  onCandidateApplied?: (snapshot: string) => void;
}): ReactElement {
  const controller = useMemo(
    () => new SpreadsheetPageController(client, project, snapshot, worksheet),
    [client, project, snapshot, worksheet],
  );
  const viewportBounds = useMemo(
    () => ({
      firstRow: 1,
      firstColumn: 1,
      lastRow: 1_048_576,
      lastColumn: 16_384,
    }),
    [],
  );
  const [selected, setSelected] = useState<SpreadsheetCellView>();
  const [capability, setCapability] = useState<ArtifactEditCapability | null>(null);
  const [diagnostic, setDiagnostic] = useState<string | null>(null);
  const [candidateId, setCandidateId] = useState<string>();
  const [candidateSummary, setCandidateSummary] = useState<string>();
  const candidateController = useMemo(
    () =>
      candidateId === undefined ? undefined : new ProjectCandidateController(client, candidateId),
    [client, candidateId],
  );
  useEffect(() => {
    setSelected(undefined);
    setCandidateId(undefined);
    setCandidateSummary(undefined);
  }, [snapshot]);
  return (
    <section aria-label="Spreadsheet worksheet">
      <h2>Worksheet {worksheet}</h2>
      <SpreadsheetPageView
        controller={controller}
        viewportBounds={viewportBounds}
        onPageLoaded={(page) => {
          setCapability(page.artifactCapability ?? null);
          setDiagnostic(page.artifactDiagnostic ?? null);
        }}
        onSelectCell={(cell, pageSnapshot) => {
          setSelected(cell);
          onSelectCell(cell, pageSnapshot);
        }}
        onSelectRange={(range, pageSnapshot) => {
          setSelected(undefined);
          setCandidateId(undefined);
          setCandidateSummary(undefined);
          onSelectRange(range, pageSnapshot);
        }}
      />
      {selected ? (
        <SpreadsheetCellEditor
          key={selected.id}
          controller={controller}
          cell={selected}
          artifactCapability={capability}
          artifactDiagnostic={diagnostic}
          onCandidateCreated={(candidate, value) => {
            setCandidateId(candidate.candidateId);
            setCandidateSummary(
              "Cell " +
                selected.row +
                ":" +
                selected.column +
                " (" +
                selected.id +
                ") changes from “" +
                reviewValue(selected.value) +
                "” to “" +
                reviewValue(value) +
                "”.",
            );
          }}
        />
      ) : null}
      {candidateController ? (
        <ProjectCandidateReview
          key={candidateId}
          controller={candidateController}
          {...(candidateSummary === undefined ? {} : { summary: candidateSummary })}
          onApplied={(result) => onCandidateApplied?.(result.successor)}
        />
      ) : null}
    </section>
  );
}
