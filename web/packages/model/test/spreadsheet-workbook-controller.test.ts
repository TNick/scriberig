/** @file Verifies workbook summaries remain bound to exact project snapshots. */
import { describe, expect, it, vi } from "vitest";
import { SpreadsheetWorkbookController } from "../src";

describe("SpreadsheetWorkbookController", () => {
  it("requests the exact workbook and returns table metadata", async () => {
    const response = {
      project: "project-1",
      snapshot: "snapshot-2",
      workbook: "workbook-3",
      worksheets: [
        {
          id: "worksheet-4",
          name: "Budget",
          usedRange: { firstRow: 1, firstColumn: 1, lastRow: 3, lastColumn: 2 },
          tables: [
            {
              id: "table-5",
              name: "BudgetTable",
              range: { firstRow: 1, firstColumn: 1, lastRow: 3, lastColumn: 2 },
            },
          ],
        },
      ],
    };
    const spreadsheetWorkbook = vi.fn(async () => response);
    const controller = new SpreadsheetWorkbookController(
      { spreadsheetWorkbook } as never,
      "project-1",
      "snapshot-2",
      "workbook-3",
    );
    const signal = new AbortController().signal;

    await expect(controller.read(signal)).resolves.toEqual(response);
    expect(spreadsheetWorkbook).toHaveBeenCalledWith(
      { project: "project-1", snapshot: "snapshot-2", workbook: "workbook-3" },
      signal,
    );
  });

  it("rejects a workbook response for a different exact snapshot", async () => {
    const controller = new SpreadsheetWorkbookController(
      {
        spreadsheetWorkbook: vi.fn(async () => ({
          project: "project-1",
          snapshot: "stale-snapshot",
          workbook: "workbook-3",
          worksheets: [],
        })),
      } as never,
      "project-1",
      "snapshot-2",
      "workbook-3",
    );

    await expect(controller.read()).rejects.toThrow("does not match the requested snapshot");
  });
});
