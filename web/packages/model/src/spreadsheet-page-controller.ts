/** @file Coordinates exact-snapshot bounded spreadsheet page requests. */
import type {
  SpreadsheetCellValue,
  SpreadsheetCellsResponse,
  SpreadsheetEditResponse,
  SpreadsheetRange,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";
import { artifactCapabilityAllowsEdit } from "./artifact-capability";

/** Controls bounded worksheet page reads over the shared semantic client.
 *
 * Public members: `read`, `next`, `previous`, `canPrevious`, `currentPage`, and
 * `proposeCellEdit`.
 * Private members: client, exact worksheet identity, page size, current page,
 * current offset, and current range.
 */
export class SpreadsheetPageController {
  private page?: SpreadsheetCellsResponse;
  private offset = 0;
  private range: SpreadsheetRange | undefined;

  /** Creates a controller bound to one exact project snapshot and worksheet.
   * @param client Client exposing the shared spreadsheet page operation.
   * @param project Stable project identity.
   * @param snapshot Exact immutable snapshot identity.
   * @param worksheet Stable worksheet semantic identity.
   * @param pageSize Maximum cells per request, from one through five hundred.
   * @throws When the page size is outside the shared API limit.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      "spreadsheetCells" | "proposeSpreadsheetCellEdit"
    >,
    private readonly project: string,
    private readonly snapshot: string,
    private readonly worksheet: string,
    private readonly pageSize = 200,
  ) {
    if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 500) {
      throw new RangeError("spreadsheet page size must be between one and five hundred");
    }
  }

  /** Reads one bounded sparse page at the bound snapshot.
   * @param range Optional inclusive rectangular bounds.
   * @param offset Zero-based cell offset.
   * @param signal Optional request cancellation signal.
   * @returns Exact-snapshot sparse cell page.
   * @throws When the response identity does not match this exact controller.
   */
  async read(
    range?: SpreadsheetRange,
    offset = 0,
    signal?: AbortSignal,
  ): Promise<SpreadsheetCellsResponse> {
    const page = await this.client.spreadsheetCells(
      {
        project: this.project,
        snapshot: this.snapshot,
        worksheet: this.worksheet,
        ...(range === undefined ? {} : { range }),
        offset,
        limit: this.pageSize,
      },
      signal,
    );
    if (
      page.project !== this.project ||
      page.snapshot !== this.snapshot ||
      page.worksheet !== this.worksheet
    ) {
      throw new TypeError("spreadsheet page identity does not match the requested snapshot");
    }
    this.range = range;
    this.offset = offset;
    this.page = page;
    return page;
  }

  /** Reads the visible row window plus a bounded number of prefetched rows.
   * @param bounds Inclusive worksheet bounds that constrain the viewport.
   * @param firstVisibleRow One-based first row currently visible.
   * @param visibleRows Number of rows visible in the grid.
   * @param prefetchRows Maximum additional rows requested ahead of the viewport.
   * @param signal Optional request cancellation signal.
   * @returns Exact-snapshot sparse cells limited to the bounded viewport range.
   * @throws When row coordinates or viewport sizes are invalid.
   */
  async readViewport(
    bounds: SpreadsheetRange,
    firstVisibleRow: number,
    visibleRows: number,
    prefetchRows = 20,
    signal?: AbortSignal,
  ): Promise<SpreadsheetCellsResponse> {
    if (
      !Number.isInteger(firstVisibleRow) ||
      !Number.isInteger(visibleRows) ||
      !Number.isInteger(prefetchRows) ||
      firstVisibleRow < 1 ||
      visibleRows < 1 ||
      prefetchRows < 0 ||
      bounds.firstRow < 1 ||
      bounds.firstColumn < 1 ||
      bounds.lastRow < bounds.firstRow ||
      bounds.lastColumn < bounds.firstColumn
    ) {
      throw new RangeError(
        "spreadsheet viewport bounds and row counts must be positive and ordered",
      );
    }
    const firstRow = Math.max(bounds.firstRow, firstVisibleRow);
    const lastRow = Math.min(bounds.lastRow, firstVisibleRow + visibleRows + prefetchRows - 1);
    if (firstRow > lastRow) {
      throw new RangeError("spreadsheet viewport does not overlap the requested worksheet bounds");
    }
    return this.read({ ...bounds, firstRow, lastRow }, 0, signal);
  }

  /** Reads the next bounded page when the current page has a continuation.
   * @param signal Optional request cancellation signal.
   * @returns The next page, or undefined when there is no current continuation.
   */
  async next(signal?: AbortSignal): Promise<SpreadsheetCellsResponse | undefined> {
    if (this.page === undefined || !this.page.hasMore) return undefined;
    return this.read(this.range, this.offset + this.page.cells.length, signal);
  }

  /** Reads the previous bounded page when the current offset is nonzero.
   * @param signal Optional request cancellation signal.
   * @returns The previous page, or undefined when already at the first page.
   */
  async previous(signal?: AbortSignal): Promise<SpreadsheetCellsResponse | undefined> {
    if (this.page === undefined || this.offset === 0) return undefined;
    return this.read(this.range, Math.max(0, this.offset - this.pageSize), signal);
  }

  /** Reports whether the current page has an earlier page.
   * @returns True when a page is loaded after offset zero.
   */
  canPrevious(): boolean {
    return this.page !== undefined && this.offset > 0;
  }

  /** Reads the most recently completed page.
   * @returns Current sparse page, if a request has completed.
   */
  currentPage(): SpreadsheetCellsResponse | undefined {
    return this.page;
  }

  /** Creates a cell edit candidate against this controller's exact snapshot.
   * @param row One-based target row.
   * @param column One-based target column.
   * @param value Authored typed value or formula.
   * @param signal Optional request cancellation signal.
   * @returns Candidate identity and validation state.
   * @throws When no exact page is loaded or its artifact is not editable.
   */
  proposeCellEdit(
    row: number,
    column: number,
    value: SpreadsheetCellValue,
    signal?: AbortSignal,
  ): Promise<SpreadsheetEditResponse> {
    if (this.page === undefined) {
      return Promise.reject(new TypeError("spreadsheet page must be loaded before editing"));
    }
    if (!artifactCapabilityAllowsEdit(this.page.artifactCapability)) {
      return Promise.reject(
        new TypeError(
          this.page.artifactDiagnostic ?? "This workbook is read-only for semantic editing.",
        ),
      );
    }
    return this.client.proposeSpreadsheetCellEdit(
      {
        project: this.project,
        snapshot: this.snapshot,
        worksheet: this.worksheet,
        row,
        column,
        value,
      },
      signal,
    );
  }
}
