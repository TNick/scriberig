/** @file Verifies public spreadsheet client operations and semantic results. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
import type { SpreadsheetRangeEditRequest } from "@scriberig/protocol";
describe("ScriberigClient spreadsheet operations", () => {
  it("reads typed sparse spreadsheet pages and normalizes formula values", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          snapshot: "snapshot-1",
          worksheet: "worksheet-1",
          artifact_capability: "read_only_derived",
          artifact_diagnostic: "Workbook fidelity is incomplete.",
          cells: [
            {
              id: "cell-1",
              row: 3,
              column: 2,
              value: {
                kind: "formula",
                value: { expression: "=A3*3", cached_value: { kind: "number", value: "30" } },
              },
              style: null,
            },
          ],
          total: 4,
          has_more: true,
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      worksheet: "worksheet-1",
      range: { firstRow: 3, firstColumn: 2, lastRow: 8, lastColumn: 5 },
      offset: 1,
      limit: 20,
    };
    const page = await new ScriberigClient(transport).spreadsheetCells(request);
    expect(page.cells[0]?.value).toEqual({
      kind: "formula",
      value: { expression: "=A3*3", cachedValue: { kind: "number", value: "30" } },
    });
    expect(page.hasMore).toBe(true);
    expect(page.artifactCapability).toBe("read_only_derived");
    expect(page.artifactDiagnostic).toBe("Workbook fidelity is incomplete.");
    expect(calls).toEqual([["spreadsheet_cells", request]]);
  });

  it("finds workbook values and preserves stable semantic cell handles", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
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
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-2",
      workbook: "workbook-3",
      query: "budget",
      offset: 0,
      limit: 10,
    };

    const result = await new ScriberigClient(transport).spreadsheetCellFind(request);

    expect(result.matches[0]).toEqual({
      worksheet: "worksheet-4",
      cell: "cell-5",
      row: 12,
      column: 3,
      authoredValue: "Budget total",
    });
    expect(result.hasMore).toBe(false);
    expect(calls).toEqual([["spreadsheet_cell_find", request]]);
  });

  it("proposes a spreadsheet edit and normalizes the reviewable candidate handle", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          candidate_id: "candidate-1",
          base_snapshot: "snapshot-1",
          path: "budget.xlsx",
          validation: { status: "pending" },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      worksheet: "worksheet-1",
      row: 3,
      column: 2,
      value: { kind: "formula", value: { expression: "=A3*4" } } as const,
    };
    const candidate = await new ScriberigClient(transport).proposeSpreadsheetCellEdit(request);
    expect(candidate).toEqual({
      project: "project-1",
      candidateId: "candidate-1",
      baseSnapshot: "snapshot-1",
      path: "budget.xlsx",
      validation: { status: "pending" },
    });
    expect(calls).toEqual([["spreadsheet_edit", request]]);
  });

  it("proposes typed range edits as one reviewable candidate", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          candidate_id: "candidate-range",
          base_snapshot: "snapshot-1",
          path: "budget.xlsx",
          validation: { status: "pending" },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      worksheet: "worksheet-1",
      edits: [
        { row: 3, column: 1, value: { kind: "number", value: "20" } },
        { row: 3, column: 2, value: { kind: "formula", value: { expression: "=A3*2" } } },
      ],
    } satisfies SpreadsheetRangeEditRequest;
    const candidate = await new ScriberigClient(transport).proposeSpreadsheetRangeEdit(request);
    expect(candidate.candidateId).toBe("candidate-range");
    expect(calls).toEqual([["spreadsheet_range_edit", request]]);
  });
});
