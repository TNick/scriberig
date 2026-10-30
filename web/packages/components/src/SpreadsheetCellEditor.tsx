/** @file Edits one exact semantic spreadsheet cell through the shared candidate API. */
import { useState, type ReactElement } from "react";
import {
  type ArtifactEditCapability,
  type SpreadsheetCellView,
  type SpreadsheetCellValue,
  type SpreadsheetEditResponse,
} from "@scriberig/protocol";
import { artifactCapabilityAllowsEdit } from "@scriberig/model";
import type { SpreadsheetPageController } from "@scriberig/model";

/** Converts the edited field text while retaining the selected cell's value kind.
 * @param cell Exact semantic cell being edited.
 * @param text Edited display text.
 * @returns Typed value to submit through the candidate API.
 */
function editedValue(cell: SpreadsheetCellView, text: string): SpreadsheetCellValue {
  switch (cell.value.kind) {
    case "blank":
      return text.length === 0 ? { kind: "blank" } : { kind: "text", value: text };
    case "text":
    case "number":
    case "error":
      return { kind: cell.value.kind, value: text };
    case "boolean":
      return { kind: "boolean", value: text.toLowerCase() === "true" };
    case "formula":
      return { kind: "formula", value: { expression: text } };
  }
}

/** Formats a typed value without treating a formula cache as its authored formula.
 * @param value Typed semantic spreadsheet value.
 * @returns Compact semantic value text.
 */
function valueText(value: SpreadsheetCellValue): string {
  if (value.kind === "blank") return "(blank)";
  if (value.kind === "formula") return value.value.expression;
  return String(value.value);
}

/** Displays typed cell details and proposes an exact-base cell edit candidate.
 * @param props Shared client and exact semantic cell target.
 * @returns Cell value editor and candidate validation result.
 */
export function SpreadsheetCellEditor({
  controller,
  cell,
  artifactCapability,
  artifactDiagnostic,
  onCandidateCreated,
}: {
  /** Exact-snapshot controller owning the shared semantic operations. */
  controller: SpreadsheetPageController;
  /** Cell selected by its stable semantic identity. */
  cell: SpreadsheetCellView;
  /** Exact artifact capability returned with the selected page. */
  artifactCapability?: ArtifactEditCapability | null;
  /** Adapter explanation for a read-only capability. */
  artifactDiagnostic?: string | null;
  /** Opens the shared review surface after candidate creation. */
  onCandidateCreated?: (candidate: SpreadsheetEditResponse, value: SpreadsheetCellValue) => void;
}): ReactElement {
  const editable = artifactCapabilityAllowsEdit(artifactCapability);
  const initial =
    cell.value.kind === "formula"
      ? cell.value.value.expression
      : String(cell.value.kind === "blank" ? "" : cell.value.value);
  const [text, setText] = useState(initial);
  const [candidate, setCandidate] = useState<string>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  /** Requests an exact-snapshot candidate for the current draft.
   * @returns Completion after candidate state or an error is displayed.
   */
  async function propose(): Promise<void> {
    setBusy(true);
    setError(undefined);
    setCandidate(undefined);
    try {
      const result = await controller.proposeCellEdit(
        cell.row,
        cell.column,
        editedValue(cell, text),
      );
      setCandidate(result.candidateId + " (" + result.validation.status + ")");
      onCandidateCreated?.(result, editedValue(cell, text));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label="Selected spreadsheet cell" data-cell-id={cell.id}>
      <h3>
        Cell {cell.row}:{cell.column}
      </h3>
      <p>Semantic cell: {cell.id}</p>
      <p>Value kind: {cell.value.kind}</p>
      {cell.value.kind === "formula" ? (
        <dl>
          <dt>Authored formula</dt>
          <dd>{cell.value.value.expression}</dd>
          <dt>Cached result</dt>
          <dd>
            {cell.value.value.cachedValue === undefined
              ? "Not supplied"
              : valueText(cell.value.value.cachedValue)}
          </dd>
        </dl>
      ) : (
        <p>Authored value: {valueText(cell.value)}</p>
      )}
      {!editable ? (
        <p role="status">
          {artifactDiagnostic ?? "This workbook is read-only for semantic editing."}
        </p>
      ) : null}
      {editable ? (
        <>
          <label>
            {cell.value.kind === "formula" ? "Formula expression" : "Cell value"}
            {cell.value.kind === "boolean" ? (
              <select value={text} onChange={(event) => setText(event.currentTarget.value)}>
                <option value="true">True</option>
                <option value="false">False</option>
              </select>
            ) : (
              <input value={text} onChange={(event) => setText(event.currentTarget.value)} />
            )}
          </label>
          <button type="button" disabled={busy || text === initial} onClick={() => void propose()}>
            Create edit candidate
          </button>
          {busy ? <p role="status">Creating candidate…</p> : null}
          {error !== undefined ? <p role="alert">{error}</p> : null}
          {candidate !== undefined ? <p role="status">Candidate {candidate}</p> : null}
        </>
      ) : null}
    </section>
  );
}
