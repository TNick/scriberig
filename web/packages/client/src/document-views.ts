/** @file Normalizes semantic document candidate responses for web consumers. */
import type {
  DocumentBlockView,
  DocumentInspectResponse,
  DocumentStyleView,
  DocumentInlineRunView,
  DocumentParagraphEditResponse,
  DocumentParagraphInsertResponse,
  DocumentParagraphView,
  DocumentTableView,
  DocumentTextStyleRef,
} from "@scriberig/protocol";
import { normalizeArtifactEditCapability } from "./artifact-views";

/** Normalizes a typed document tree from Rust wire names.
 * @param value Raw transport response.
 * @returns Snapshot-bound ordered semantic document tree.
 * @throws When a nested section, paragraph, run, or table is malformed.
 */
export function normalizeDocumentInspectResponse(value: unknown): DocumentInspectResponse {
  const response = record(value, "document tree");
  if (!Array.isArray(response.sections)) throw new TypeError("document sections must be an array");
  const rawStyles = response.styles ?? [];
  if (!Array.isArray(rawStyles)) throw new TypeError("document styles must be an array");
  const artifactCapability = normalizeArtifactEditCapability(
    field(response, "artifact_capability", "artifactCapability"),
  );
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    document: string(response.document, "document id"),
    ...(artifactCapability === undefined ? {} : { artifactCapability }),
    styles: rawStyles.map(normalizeDocumentStyle),
    sections: response.sections.map((value) => {
      const section = record(value, "document section");
      if (!Array.isArray(section.blocks)) throw new TypeError("document blocks must be an array");
      return {
        id: string(section.id, "section id"),
        blocks: section.blocks.map(normalizeBlock),
      };
    }),
  };
}

/** Normalizes one named document style.
 * @param value Raw transport style.
 * @returns Typed adapter-owned style definition.
 * @throws When the style key or optional name is malformed.
 */
function normalizeDocumentStyle(value: unknown): DocumentStyleView {
  const style = record(value, "document style");
  return {
    key: string(style.key, "style key"),
    ...(style.name == null ? {} : { name: string(style.name, "style name") }),
  };
}

/** Normalizes one tagged paragraph or table block.
 * @param value Raw block value.
 * @returns Typed semantic block.
 * @throws When the block tag or payload is invalid.
 */
function normalizeBlock(value: unknown): DocumentBlockView {
  const block = record(value, "document block");
  const kind = string(block.kind, "block kind");
  if (kind === "paragraph") return { kind, paragraph: normalizeParagraph(block.paragraph) };
  if (kind === "table") return { kind, table: normalizeTable(block.table) };
  throw new TypeError("document block kind is invalid");
}

/** Normalizes one paragraph and its authored inline run order.
 * @param value Raw paragraph value.
 * @returns Typed semantic paragraph.
 * @throws When paragraph identity or runs are invalid.
 */
function normalizeParagraph(value: unknown): DocumentParagraphView {
  const paragraph = record(value, "document paragraph");
  if (!Array.isArray(paragraph.runs)) throw new TypeError("paragraph runs must be an array");
  const style = normalizeStyle(paragraph.style);
  return {
    id: string(paragraph.id, "paragraph id"),
    runs: paragraph.runs.map(normalizeRun),
    ...(style === undefined ? {} : { style }),
  };
}

/** Normalizes one inline run and its optional style/link references.
 * @param value Raw run value.
 * @returns Typed semantic run.
 * @throws When run fields are malformed.
 */
function normalizeRun(value: unknown): DocumentInlineRunView {
  const run = record(value, "document run");
  const style = normalizeStyle(run.style);
  return {
    id: string(run.id, "run id"),
    text: string(run.text, "run text"),
    ...(style === undefined ? {} : { style }),
    ...(run.hyperlink == null ? {} : { hyperlink: string(run.hyperlink, "hyperlink") }),
  };
}

/** Normalizes one recursively nested table.
 * @param value Raw table value.
 * @returns Typed ordered table.
 * @throws When rows, cells, or nested block arrays are malformed.
 */
function normalizeTable(value: unknown): DocumentTableView {
  const table = record(value, "document table");
  if (!Array.isArray(table.rows)) throw new TypeError("table rows must be an array");
  return {
    id: string(table.id, "table id"),
    rows: table.rows.map((value) => {
      const row = record(value, "document row");
      if (!Array.isArray(row.cells)) throw new TypeError("row cells must be an array");
      return {
        id: string(row.id, "row id"),
        cells: row.cells.map((value) => {
          const cell = record(value, "document cell");
          if (!Array.isArray(cell.blocks)) throw new TypeError("cell blocks must be an array");
          return {
            id: string(cell.id, "cell id"),
            blocks: cell.blocks.map(normalizeBlock),
          };
        }),
      };
    }),
  };
}

/** Normalizes an optional style reference.
 * @param value Raw style value.
 * @returns Typed style reference, or undefined when absent.
 * @throws When a present style is malformed.
 */
function normalizeStyle(value: unknown): DocumentTextStyleRef | undefined {
  if (value == null) return undefined;
  const style = record(value, "document style");
  return {
    key: string(style.key, "style key"),
    adapter: string(style.adapter, "style adapter"),
  };
}

/** Normalizes a document candidate response from Rust wire field names.
 * @param value Raw transport response.
 * @returns Candidate metadata using the public camel-case model.
 * @throws When candidate or validation fields are malformed.
 */
export function normalizeDocumentParagraphEditResponse(
  value: unknown,
): DocumentParagraphEditResponse {
  const response = record(value, "document candidate");
  const validation = record(response.validation, "document validation");
  const status = string(validation.status, "validation status");
  const common = {
    project: string(response.project, "project"),
    candidateId: string(field(response, "candidate_id", "candidateId"), "candidate id"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base snapshot"),
    path: string(response.path, "path"),
  };
  if (status === "pending" || status === "passed") return { ...common, validation: { status } };
  if (status === "failed" && Array.isArray(validation.diagnostics)) {
    return {
      ...common,
      validation: {
        status,
        diagnostics: validation.diagnostics.map((item) => string(item, "diagnostic")),
      },
    };
  }
  throw new TypeError("document validation status is invalid");
}

/** Normalizes a document paragraph insertion response from Rust wire fields.
 * @param value Raw transport response.
 * @returns Candidate metadata using the public camel-case model.
 * @throws When candidate or validation fields are malformed.
 */
export function normalizeDocumentParagraphInsertResponse(
  value: unknown,
): DocumentParagraphInsertResponse {
  return normalizeDocumentParagraphEditResponse(value);
}

/** Requires a non-null object record.
 * @param value Candidate value.
 * @param label Diagnostic context.
 * @returns Object record.
 * @throws When value is not a record.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Reads snake-case server or camel-case public field spelling.
 * @param source Object containing the field.
 * @param snake Rust wire field name.
 * @param camel Public model field name.
 * @returns Present field value.
 */
function field(source: Record<string, unknown>, snake: string, camel: string): unknown {
  return source[snake] ?? source[camel];
}

/** Requires a string field.
 * @param value Candidate value.
 * @param name Field name for diagnostics.
 * @returns Validated string.
 * @throws When value is not a string.
 */
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new TypeError(`document ${name} must be a string`);
  return value;
}
