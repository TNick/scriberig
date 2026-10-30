/** @file Provides snapshot-bound spreadsheet operations over a semantic transport. */
import type {
  SpreadsheetCellsRequest,
  SpreadsheetCellsResponse,
  SpreadsheetCellFindRequest,
  SpreadsheetCellFindResponse,
  SpreadsheetEditRequest,
  SpreadsheetEditResponse,
  SpreadsheetRangeEditRequest,
  SpreadsheetRenameRequest,
  SpreadsheetWorkbookRequest,
  SpreadsheetWorkbookResponse,
} from "@scriberig/protocol";
import {
  normalizeSpreadsheetCellsResponse,
  normalizeSpreadsheetCellFindResponse,
  normalizeSpreadsheetEditResponse,
  normalizeSpreadsheetWorkbookResponse,
} from "./spreadsheet-views";
import type { SemanticTransport } from "./transport";
import { ArtifactClientOperations } from "./artifact-client-operations";

/** Owns typed spreadsheet operations shared by semantic client surfaces.
 *
 * Public members: constructor and spreadsheet read/find/edit/rename methods.
 * Private members: none.
 */
export class SpreadsheetClientOperations extends ArtifactClientOperations {
  /** Creates spreadsheet operations over an injected transport.
   * @param transport Protocol transport with snapshot-bound server operations.
   */
  constructor(transport: SemanticTransport) {
    super(transport);
  }

  /** Reads a bounded sparse worksheet page at an exact snapshot.
   * @param request Exact project/snapshot/worksheet and page selection.
   * @param signal Optional request cancellation signal.
   * @returns Typed sparse cell page and continuation metadata.
   */
  spreadsheetCells(
    request: SpreadsheetCellsRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetCellsResponse> {
    return this.transport
      .request("spreadsheet_cells", request, signal)
      .then(normalizeSpreadsheetCellsResponse);
  }

  /** Reads exact-snapshot worksheet and named-table summaries for a workbook.
   * @param request Exact project, snapshot, and workbook identities.
   * @param signal Optional request cancellation signal.
   * @returns Ordered worksheet summaries with bounded ranges and table identities.
   */
  spreadsheetWorkbook(
    request: SpreadsheetWorkbookRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetWorkbookResponse> {
    return this.transport
      .request("spreadsheet_workbook", request, signal)
      .then(normalizeSpreadsheetWorkbookResponse);
  }

  /** Finds authored cell values at an exact workbook snapshot.
   * @param request Exact workbook snapshot, query, and page selection.
   * @param signal Optional request cancellation signal.
   * @returns Bounded semantic matches with stable cell and worksheet handles.
   */
  spreadsheetCellFind(
    request: SpreadsheetCellFindRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetCellFindResponse> {
    return this.transport
      .request("spreadsheet_cell_find", request, signal)
      .then(normalizeSpreadsheetCellFindResponse);
  }

  /** Creates an exact-base spreadsheet candidate through shared operations.
   * @param request Exact target and authored value/formula.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeSpreadsheetCellEdit(
    request: SpreadsheetEditRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetEditResponse> {
    return this.transport
      .request("spreadsheet_edit", request, signal)
      .then(normalizeSpreadsheetEditResponse);
  }

  /** Creates one exact-base candidate containing multiple cell updates.
   * @param request Exact worksheet snapshot and bounded set of typed updates.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeSpreadsheetRangeEdit(
    request: SpreadsheetRangeEditRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetEditResponse> {
    return this.transport
      .request("spreadsheet_range_edit", request, signal)
      .then(normalizeSpreadsheetEditResponse);
  }

  /** Creates an exact-base worksheet rename candidate through shared operations.
   * @param request Exact worksheet identity and requested name.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeSpreadsheetRename(
    request: SpreadsheetRenameRequest,
    signal?: AbortSignal,
  ): Promise<SpreadsheetEditResponse> {
    return this.transport
      .request("spreadsheet_rename", request, signal)
      .then(normalizeSpreadsheetEditResponse);
  }
}
