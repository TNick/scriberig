/** @file Defines bounded snapshot-bound spreadsheet wire contracts. */

import type { ArtifactEditCapability } from "./artifacts";

/** Identifies an inclusive one-based rectangular worksheet range. */
export interface SpreadsheetRange {
  /** First row, inclusive. */
  firstRow: number;
  /** First column, inclusive. */
  firstColumn: number;
  /** Last row, inclusive. */
  lastRow: number;
  /** Last column, inclusive. */
  lastColumn: number;
}

/** Requests a bounded worksheet cell page from an exact semantic snapshot. */
export interface SpreadsheetCellsRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** Optional inclusive rectangular bounds. */
  range?: SpreadsheetRange;
  /** Zero-based page offset. */
  offset: number;
  /** Maximum number of cells, from one through five hundred. */
  limit: number;
}

/** Requests ordered worksheet summaries at an exact workbook snapshot. */
export interface SpreadsheetWorkbookRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable workbook semantic identity. */
  workbook: string;
}

/** Represents one typed spreadsheet value. */
export type SpreadsheetCellValue =
  | { kind: "blank" }
  | { kind: "text"; value: string }
  | { kind: "number"; value: string }
  | { kind: "boolean"; value: boolean }
  | { kind: "error"; value: string }
  | {
      kind: "formula";
      value: { expression: string; cachedValue?: SpreadsheetCellValue };
    };

/** Describes one sparse semantic cell. */
export interface SpreadsheetCellView {
  /** Stable Scriberig cell identity. */
  id: string;
  /** One-based row coordinate. */
  row: number;
  /** One-based column coordinate. */
  column: number;
  /** Authored value or formula. */
  value: SpreadsheetCellValue;
  /** Adapter-neutral style identity when present. */
  style?: { key: string; adapter: string };
}

/** Returns one sparse worksheet page from an exact snapshot. */
export interface SpreadsheetCellsResponse {
  /** Stable project identity. */
  project: string;
  /** Exact snapshot used for the cell revisions. */
  snapshot: string;
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** Capability of the exact source artifact, when catalogued. */
  artifactCapability?: ArtifactEditCapability | null;
  /** Adapter diagnostic explaining restricted edit capability. */
  artifactDiagnostic?: string | null;
  /** Sparse cells ordered by row then column. */
  cells: SpreadsheetCellView[];
  /** Total matching sparse cells before pagination. */
  total: number;
  /** Whether more cells remain after this page. */
  hasMore: boolean;
}

/** Identifies one semantic table and its inclusive worksheet range. */
export interface SpreadsheetTableSummary {
  /** Stable semantic table identity. */
  id: string;
  /** Authored table name. */
  name: string;
  /** Exact one-based table bounds. */
  range: SpreadsheetRange;
}

/** Summarizes one worksheet at an exact workbook snapshot. */
export interface SpreadsheetWorksheetSummary {
  /** Stable semantic worksheet identity. */
  id: string;
  /** Authored worksheet name. */
  name: string;
  /** Bounded authored cell extent, when present. */
  usedRange?: SpreadsheetRange;
  /** Named tables included in the exact worksheet revision. */
  tables: SpreadsheetTableSummary[];
}

/** Returns workbook and ordered worksheet summaries at one exact snapshot. */
export interface SpreadsheetWorkbookResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity used by the read. */
  snapshot: string;
  /** Stable workbook semantic identity. */
  workbook: string;
  /** Capability of the exact source artifact, when catalogued. */
  artifactCapability?: ArtifactEditCapability | null;
  /** Worksheets in workbook collection order. */
  worksheets: SpreadsheetWorksheetSummary[];
}

/** Requests bounded authored-value search in a workbook at an exact snapshot. */
export interface SpreadsheetCellFindRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable workbook semantic identity. */
  workbook: string;
  /** Non-empty case-insensitive authored-value substring. */
  query: string;
  /** Zero-based match offset. */
  offset: number;
  /** Maximum matches, from one through one hundred. */
  limit: number;
}

/** Identifies one authored spreadsheet cell match. */
export interface SpreadsheetCellMatch {
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** Stable cell semantic identity. */
  cell: string;
  /** One-based cell row. */
  row: number;
  /** One-based cell column. */
  column: number;
  /** Authored value or formula expression that matched. */
  authoredValue: string;
}

/** Returns bounded exact-snapshot semantic cell matches. */
export interface SpreadsheetCellFindResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot used for every match. */
  snapshot: string;
  /** Stable workbook semantic identity. */
  workbook: string;
  /** Matching semantic cell handles in workbook order. */
  matches: SpreadsheetCellMatch[];
  /** Total matches before pagination. */
  total: number;
  /** Whether another result page exists. */
  hasMore: boolean;
}

/** Requests an exact-base spreadsheet cell edit candidate. */
export interface SpreadsheetEditRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** One-based target row. */
  row: number;
  /** One-based target column. */
  column: number;
  /** New authored value or formula. */
  value: SpreadsheetCellValue;
}

/** One typed cell update in a bounded range edit. */
export interface SpreadsheetCellEdit {
  /** One-based target row. */
  row: number;
  /** One-based target column. */
  column: number;
  /** Authored value or formula. */
  value: SpreadsheetCellValue;
}

/** Requests one candidate containing a bounded set of spreadsheet cell updates. */
export interface SpreadsheetRangeEditRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** One through five hundred distinct cell updates. */
  edits: SpreadsheetCellEdit[];
}

/** Requests an exact-base semantic worksheet rename candidate. */
export interface SpreadsheetRenameRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable worksheet semantic identity. */
  worksheet: string;
  /** Requested authored worksheet name. */
  new_name: string;
}

/** Returns the candidate handle and validation state for a spreadsheet edit. */
export interface SpreadsheetEditResponse {
  /** Stable project identity. */
  project: string;
  /** Candidate identity for shared validation and review. */
  candidateId: string;
  /** Exact base snapshot used by the candidate. */
  baseSnapshot: string;
  /** Project-relative workbook path. */
  path: string;
  /** Current candidate validation state. */
  validation:
    { status: "pending" } | { status: "passed" } | { status: "failed"; diagnostics: string[] };
}
