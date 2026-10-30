/** @file Connects bounded spreadsheet page control to semantic cell rendering. */
import { useEffect, useState, type ReactElement } from "react";
import type { SpreadsheetCellView, SpreadsheetRange } from "@scriberig/protocol";
import type { SpreadsheetPageController } from "@scriberig/model";
import type { SpreadsheetCellsResponse } from "@scriberig/protocol";
import { SpreadsheetCellTable } from "./SpreadsheetCellTable";

/** Loads and pages sparse semantic worksheet cells through a shared controller.
 * @param props Page controller, optional range, and semantic selection callback.
 * @returns Bounded table with page controls and loading/error status.
 */
export function SpreadsheetPageView({
  controller,
  range,
  viewportBounds,
  viewportRows = 40,
  prefetchRows = 20,
  onSelectCell,
  onSelectRange,
  onPageLoaded,
}: {
  /** Exact-snapshot controller backed by the public semantic client. */
  controller: SpreadsheetPageController;
  /** Optional inclusive rectangle to constrain the sparse page results. */
  range?: SpreadsheetRange;
  /** Worksheet bounds used to request a row viewport instead of the full range. */
  viewportBounds?: SpreadsheetRange;
  /** Number of worksheet rows shown in one viewport request. */
  viewportRows?: number;
  /** Additional rows fetched ahead of the viewport. */
  prefetchRows?: number;
  /** Optional callback receiving the selected semantic cell identity. */
  onSelectCell?: (cell: SpreadsheetCellView, snapshot: string) => void;
  /** Optional callback receiving an inclusive worksheet range selection. */
  onSelectRange?: (range: SpreadsheetRange, snapshot: string) => void;
  /** Publishes exact page metadata, including artifact edit capability. */
  onPageLoaded?: (page: SpreadsheetCellsResponse) => void;
}): ReactElement {
  const [page, setPage] = useState(controller.currentPage());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();
  const [firstVisibleRow, setFirstVisibleRow] = useState(viewportBounds?.firstRow ?? 1);

  const readViewport = (row: number, signal?: AbortSignal) =>
    viewportBounds === undefined
      ? controller.read(range, 0, signal)
      : controller.readViewport(viewportBounds, row, viewportRows, prefetchRows, signal);

  useEffect(() => {
    const request = new AbortController();
    setPage(undefined);
    setError(undefined);
    setLoading(true);
    setFirstVisibleRow(viewportBounds?.firstRow ?? 1);
    void readViewport(viewportBounds?.firstRow ?? 1, request.signal)
      .then((result) => {
        if (!request.signal.aborted) {
          setPage(result);
          onPageLoaded?.(result);
        }
      })
      .catch((reason: unknown) => {
        if (!request.signal.aborted) setError(String(reason));
      })
      .finally(() => {
        if (!request.signal.aborted) setLoading(false);
      });
    return () => request.abort();
  }, [controller, range, viewportBounds, viewportRows, prefetchRows]);

  return (
    <section aria-label="Spreadsheet page">
      {viewportBounds !== undefined ? (
        <form
          aria-label="Spreadsheet row viewport"
          onSubmit={(event) => {
            event.preventDefault();
            if (!Number.isInteger(firstVisibleRow) || firstVisibleRow < 1) {
              setError("Enter a positive first row number.");
              return;
            }
            setLoading(true);
            setError(undefined);
            void readViewport(firstVisibleRow)
              .then((result) => {
                setPage(result);
                onPageLoaded?.(result);
              })
              .catch((reason: unknown) => setError(String(reason)))
              .finally(() => setLoading(false));
          }}
        >
          <label>
            First visible row
            <input
              type="number"
              min={viewportBounds.firstRow}
              max={viewportBounds.lastRow}
              value={firstVisibleRow}
              onChange={(event) => setFirstVisibleRow(Number(event.currentTarget.value))}
            />
          </label>
          <button type="submit" disabled={loading}>
            Load row window
          </button>
          <span>
            Shows up to {viewportRows} rows with {prefetchRows} rows of prefetch.
          </span>
        </form>
      ) : null}
      {loading ? <p role="status">Loading cells…</p> : null}
      {error !== undefined ? <p role="alert">{error}</p> : null}
      {page !== undefined ? (
        <SpreadsheetCellTable
          page={page}
          {...(onSelectCell === undefined
            ? {}
            : {
                onSelectCell: (cellId: string) => {
                  const cell = page.cells.find((candidate) => candidate.id === cellId);
                  if (cell !== undefined) onSelectCell(cell, page.snapshot);
                },
              })}
          {...(onSelectRange === undefined
            ? {}
            : { onSelectRange: (selectedRange) => onSelectRange(selectedRange, page.snapshot) })}
        />
      ) : null}
      {page !== undefined ? (
        <nav aria-label="Spreadsheet pages">
          <button
            type="button"
            disabled={loading || !controller.canPrevious()}
            onClick={() => {
              setLoading(true);
              setError(undefined);
              void controller
                .previous()
                .then((result) => {
                  if (result !== undefined) setPage(result);
                  if (result !== undefined) onPageLoaded?.(result);
                })
                .catch((reason: unknown) => setError(String(reason)))
                .finally(() => setLoading(false));
            }}
          >
            Previous cells
          </button>
          <button
            type="button"
            disabled={loading || !page.hasMore}
            onClick={() => {
              setLoading(true);
              setError(undefined);
              void controller
                .next()
                .then((result) => {
                  if (result !== undefined) setPage(result);
                  if (result !== undefined) onPageLoaded?.(result);
                })
                .catch((reason: unknown) => setError(String(reason)))
                .finally(() => setLoading(false));
            }}
          >
            Next cells
          </button>
        </nav>
      ) : null}
    </section>
  );
}
