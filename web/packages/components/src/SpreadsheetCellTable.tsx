/** @file Renders one bounded page of semantic spreadsheet cells. */
import { useEffect, useState, type ReactElement } from "react";
import type {
  SpreadsheetCellsResponse,
  SpreadsheetCellValue,
  SpreadsheetRange,
} from "@scriberig/protocol";

/** Formats a semantic spreadsheet value for compact display.
 * @param value Typed value from the semantic protocol.
 * @returns Authored formula text or the scalar display value.
 */
function formatCellValue(value: SpreadsheetCellValue): string {
  if (value.kind === "blank") return "";
  if (value.kind === "formula") return value.value.expression;
  return String(value.value);
}

/** Converts a one-based column index to its spreadsheet label.
 * @param column Positive one-based column coordinate.
 * @returns Spreadsheet column label.
 */
function columnLabel(column: number): string {
  let value = column;
  let label = "";
  while (value > 0) {
    value -= 1;
    label = String.fromCharCode(65 + (value % 26)) + label;
    value = Math.floor(value / 26);
  }
  return label;
}

/** Renders a bounded sparse cell page with stable semantic cell targets.
 * @param props Exact page response and optional cell selection callback.
 * @returns Accessible table containing only cells in the supplied page.
 */
export function SpreadsheetCellTable({
  page,
  onSelectCell,
  onSelectRange,
}: {
  /** Snapshot-bound sparse cell page. */
  page: SpreadsheetCellsResponse;
  /** Optional callback for selecting a semantic cell identity. */
  onSelectCell?: (cellId: string) => void;
  /** Optional callback for selecting a rectangular range with Shift-click. */
  onSelectRange?: (range: SpreadsheetRange) => void;
}): ReactElement {
  const [rangeOrigin, setRangeOrigin] = useState<{
    row: number;
    column: number;
    snapshot: string;
    worksheet: string;
  }>();
  useEffect(() => setRangeOrigin(undefined), [page.snapshot, page.worksheet]);
  return (
    <section aria-label="Spreadsheet cells" data-snapshot={page.snapshot}>
      <table>
        <caption>
          Worksheet {page.worksheet}: {page.cells.length} of {page.total} cells
        </caption>
        <thead>
          <tr>
            <th scope="col">Cell</th>
            <th scope="col">Value</th>
            <th scope="col">Kind</th>
          </tr>
        </thead>
        <tbody>
          {page.cells.map((cell) => {
            const address = `${columnLabel(cell.column)}${cell.row}`;
            const content = formatCellValue(cell.value);
            return (
              <tr key={cell.id} data-cell-id={cell.id}>
                <th scope="row">
                  {onSelectCell ? (
                    <button
                      type="button"
                      onClick={(event) => {
                        if (
                          event.shiftKey &&
                          rangeOrigin?.snapshot === page.snapshot &&
                          rangeOrigin.worksheet === page.worksheet &&
                          onSelectRange
                        ) {
                          onSelectRange({
                            firstRow: Math.min(rangeOrigin.row, cell.row),
                            firstColumn: Math.min(rangeOrigin.column, cell.column),
                            lastRow: Math.max(rangeOrigin.row, cell.row),
                            lastColumn: Math.max(rangeOrigin.column, cell.column),
                          });
                          return;
                        }
                        setRangeOrigin({
                          row: cell.row,
                          column: cell.column,
                          snapshot: page.snapshot,
                          worksheet: page.worksheet,
                        });
                        onSelectCell(cell.id);
                      }}
                    >
                      {address}
                    </button>
                  ) : (
                    address
                  )}
                </th>
                <td>{content}</td>
                <td>{cell.value.kind}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {page.hasMore ? <p role="status">More cells are available.</p> : null}
    </section>
  );
}
