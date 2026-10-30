/** @file Normalizes bounded spreadsheet cell pages for public client consumers. */
import type {
  SpreadsheetCellValue,
  SpreadsheetCellFindResponse,
  SpreadsheetCellView,
  SpreadsheetCellsResponse,
  SpreadsheetEditResponse,
  SpreadsheetRange,
  SpreadsheetTableSummary,
  SpreadsheetWorkbookResponse,
} from "@scriberig/protocol";
import { normalizeArtifactEditCapability } from "./artifact-views";

/** Normalizes snake-case server fields into a validated spreadsheet page.
 * @param value Untyped response returned by the transport.
 * @returns Validated snapshot-bound sparse cell page.
 * @throws When required response fields or typed cells are invalid.
 */
export function normalizeSpreadsheetCellsResponse(value: unknown): SpreadsheetCellsResponse {
  const response = record(value, "spreadsheet response");
  if (!Array.isArray(response.cells)) throw new TypeError("spreadsheet cells must be an array");
  const artifactCapability = normalizeArtifactEditCapability(
    field(response, "artifact_capability", "artifactCapability"),
  );
  const diagnostic = field(response, "artifact_diagnostic", "artifactDiagnostic");
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    worksheet: string(response.worksheet, "worksheet"),
    ...(artifactCapability === undefined ? {} : { artifactCapability }),
    ...(diagnostic === undefined || diagnostic === null
      ? {}
      : { artifactDiagnostic: string(diagnostic, "artifact diagnostic") }),
    cells: response.cells.map(normalizeCell),
    total: number(response.total, "total"),
    hasMore: boolean(field(response, "has_more", "hasMore"), "has_more"),
  };
}

/** Normalizes exact-snapshot worksheet summaries and table ranges.
 * @param value Untyped response returned by the transport.
 * @returns Workbook metadata with stable worksheet and table identities.
 * @throws When a required workbook, worksheet, or table field is malformed.
 */
export function normalizeSpreadsheetWorkbookResponse(value: unknown): SpreadsheetWorkbookResponse {
  const response = record(value, "spreadsheet workbook response");
  if (!Array.isArray(response.worksheets)) {
    throw new TypeError("spreadsheet worksheets must be an array");
  }
  const artifactCapability = normalizeArtifactEditCapability(
    field(response, "artifact_capability", "artifactCapability"),
  );
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    workbook: string(response.workbook, "workbook"),
    ...(artifactCapability === undefined ? {} : { artifactCapability }),
    worksheets: response.worksheets.map(normalizeWorksheet),
  };
}

/** Normalizes exact-snapshot spreadsheet search matches.
 * @param value Untyped response returned by the transport.
 * @returns Bounded semantic cell matches with stable identities.
 * @throws When required snapshot, match, or pagination fields are malformed.
 */
export function normalizeSpreadsheetCellFindResponse(value: unknown): SpreadsheetCellFindResponse {
  const response = record(value, "spreadsheet cell find response");
  if (!Array.isArray(response.matches)) throw new TypeError("spreadsheet matches must be an array");
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    workbook: string(response.workbook, "workbook"),
    matches: response.matches.map((entry) => {
      const match = record(entry, "spreadsheet cell match");
      return {
        worksheet: string(match.worksheet, "worksheet"),
        cell: string(match.cell, "cell"),
        row: number(match.row, "row"),
        column: number(match.column, "column"),
        authoredValue: string(field(match, "authored_value", "authoredValue"), "authored_value"),
      };
    }),
    total: number(response.total, "total"),
    hasMore: boolean(field(response, "has_more", "hasMore"), "has_more"),
  };
}

/** Normalizes one worksheet summary from Rust serde or public camel-case data.
 * @param value Untyped worksheet object.
 * @returns Typed worksheet identity, extent, and named tables.
 * @throws When a required worksheet or table field is malformed.
 */
function normalizeWorksheet(value: unknown): SpreadsheetWorkbookResponse["worksheets"][number] {
  const worksheet = record(value, "spreadsheet worksheet");
  if (!Array.isArray(worksheet.tables)) throw new TypeError("spreadsheet tables must be an array");
  const usedRange = field(worksheet, "used_range", "usedRange");
  return {
    id: string(worksheet.id, "worksheet id"),
    name: string(worksheet.name, "worksheet name"),
    ...(usedRange === null || usedRange === undefined
      ? {}
      : { usedRange: normalizeRange(usedRange) }),
    tables: worksheet.tables.map(normalizeTable),
  };
}

/** Normalizes one semantic table summary.
 * @param value Untyped table object.
 * @returns Stable table identity, name, and range.
 * @throws When any required table field is malformed.
 */
function normalizeTable(value: unknown): SpreadsheetTableSummary {
  const table = record(value, "spreadsheet table");
  return {
    id: string(table.id, "table id"),
    name: string(table.name, "table name"),
    range: normalizeRange(table.range),
  };
}

/** Normalizes one one-based rectangular range from serde or public spelling.
 * @param value Untyped range object.
 * @returns Validated inclusive cell range.
 * @throws When coordinates are missing or nonnumeric.
 */
function normalizeRange(value: unknown): SpreadsheetRange {
  const range = record(value, "spreadsheet range");
  return {
    firstRow: number(field(range, "first_row", "firstRow"), "first row"),
    firstColumn: number(field(range, "first_column", "firstColumn"), "first column"),
    lastRow: number(field(range, "last_row", "lastRow"), "last row"),
    lastColumn: number(field(range, "last_column", "lastColumn"), "last column"),
  };
}

/** Normalizes a spreadsheet candidate response from the Rust wire spelling.
 * @param value Raw transport response.
 * @returns Candidate metadata in the public camel-case model.
 * @throws When required candidate or validation fields are malformed.
 */
export function normalizeSpreadsheetEditResponse(value: unknown): SpreadsheetEditResponse {
  const response = record(value, "spreadsheet edit response");
  const validation = record(response.validation, "spreadsheet validation");
  const status = string(validation.status, "validation status");
  const common = {
    project: string(response.project, "project"),
    candidateId: string(field(response, "candidate_id", "candidateId"), "candidate_id"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base_snapshot"),
    path: string(response.path, "path"),
  };
  if (status === "pending" || status === "passed") {
    return { ...common, validation: { status } };
  }
  if (status === "failed" && Array.isArray(validation.diagnostics)) {
    return {
      ...common,
      validation: {
        status,
        diagnostics: validation.diagnostics.map((item) => string(item, "diagnostic")),
      },
    };
  }
  throw new TypeError("spreadsheet validation status is invalid");
}

/** Normalizes one typed sparse cell.
 * @param value Untyped cell object.
 * @returns Validated cell view.
 * @throws When the cell identity, coordinates, or value are invalid.
 */
function normalizeCell(value: unknown): SpreadsheetCellView {
  const cell = record(value, "spreadsheet cell");
  const style =
    cell.style === null || cell.style === undefined ? undefined : normalizeStyle(cell.style);
  return {
    id: string(cell.id, "cell id"),
    row: number(cell.row, "row"),
    column: number(cell.column, "column"),
    value: normalizeValue(cell.value),
    ...(style === undefined ? {} : { style }),
  };
}

/** Normalizes one optional adapter-neutral cell style.
 * @param value Untyped style object.
 * @returns Validated style identity.
 * @throws When either style field is invalid.
 */
function normalizeStyle(value: unknown): { key: string; adapter: string } {
  const style = record(value, "cell style");
  return { key: string(style.key, "style key"), adapter: string(style.adapter, "style adapter") };
}

/** Normalizes a discriminated cell value, including advisory formula cache.
 * @param value Untyped wire value.
 * @returns Validated semantic cell value.
 * @throws When the value tag or its payload is invalid.
 */
function normalizeValue(value: unknown): SpreadsheetCellValue {
  const item = record(value, "cell value");
  const kind = string(item.kind, "cell value kind");
  if (kind === "blank") return { kind };
  if (kind === "text" || kind === "number" || kind === "error") {
    return { kind, value: string(item.value, "cell scalar") };
  }
  if (kind === "boolean") return { kind, value: boolean(item.value, "cell boolean") };
  if (kind === "formula") {
    const formula = record(item.value, "formula value");
    const cached = field(formula, "cached_value", "cachedValue");
    return {
      kind,
      value: {
        expression: string(formula.expression, "formula expression"),
        ...(cached === null || cached === undefined ? {} : { cachedValue: normalizeValue(cached) }),
      },
    };
  }
  throw new TypeError(`unsupported spreadsheet cell value kind: ${kind}`);
}

/** Requires a non-null object record.
 * @param value Candidate value.
 * @param label Diagnostic context.
 * @returns Object record.
 * @throws When the candidate is not an object.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Reads either server snake-case or client camel-case spelling.
 * @param source Object containing the field.
 * @param snake Server field spelling.
 * @param camel Public client field spelling.
 * @returns Present field value.
 */
function field(source: Record<string, unknown>, snake: string, camel: string): unknown {
  return source[snake] ?? source[camel];
}

/** Requires a string field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated string.
 * @throws When the field is not a string.
 */
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new TypeError(`spreadsheet ${name} must be a string`);
  return value;
}

/** Requires a finite numeric field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated number.
 * @throws When the field is not finite numeric data.
 */
function number(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`spreadsheet ${name} must be a finite number`);
  }
  return value;
}

/** Requires a boolean field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated boolean.
 * @throws When the field is not a boolean.
 */
function boolean(value: unknown, name: string): boolean {
  if (typeof value !== "boolean") throw new TypeError(`spreadsheet ${name} must be a boolean`);
  return value;
}
