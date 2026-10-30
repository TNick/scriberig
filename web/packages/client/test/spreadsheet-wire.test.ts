/** @file Verifies spreadsheet protocol wire encoding and response normalization. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";
describe("spreadsheet protocol wire contracts", () => {
  it("encodes spreadsheet search using the Rust exact-snapshot contract", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        workbook: "workbook-3",
        query: "budget",
        offset: 0,
        limit: 10,
      });
      return new Response(
        JSON.stringify({
          response: {
            SpreadsheetCellFind: {
              project: "project-1",
              snapshot: "snapshot-2",
              workbook: "workbook-3",
              matches: [
                {
                  worksheet: "worksheet-4",
                  cell: "cell-5",
                  row: 12,
                  column: 3,
                  authored_value: "Budget total",
                },
              ],
              total: 1,
              has_more: false,
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const result = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).spreadsheetCellFind({
      project: "project-1",
      snapshot: "snapshot-2",
      workbook: "workbook-3",
      query: "budget",
      offset: 0,
      limit: 10,
    });

    expect(result.matches[0]?.authoredValue).toBe("Budget total");
    expect(result.snapshot).toBe("snapshot-2");
    vi.unstubAllGlobals();
  });

  it("normalizes exact-snapshot workbook and semantic table summaries", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        workbook: "workbook-3",
      });
      return new Response(
        JSON.stringify({
          response: {
            SpreadsheetWorkbook: {
              project: "project-1",
              snapshot: "snapshot-2",
              workbook: "workbook-3",
              worksheets: [
                {
                  id: "worksheet-4",
                  name: "Budget",
                  used_range: {
                    firstRow: 1,
                    firstColumn: 1,
                    lastRow: 3,
                    lastColumn: 2,
                  },
                  tables: [
                    {
                      id: "table-5",
                      name: "BudgetTable",
                      range: {
                        firstRow: 1,
                        firstColumn: 1,
                        lastRow: 3,
                        lastColumn: 2,
                      },
                    },
                  ],
                },
              ],
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const workbook = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).spreadsheetWorkbook({
      project: "project-1",
      snapshot: "snapshot-2",
      workbook: "workbook-3",
    });

    expect(workbook.worksheets[0]?.tables[0]).toEqual({
      id: "table-5",
      name: "BudgetTable",
      range: { firstRow: 1, firstColumn: 1, lastRow: 3, lastColumn: 2 },
    });
    vi.unstubAllGlobals();
  });
});
