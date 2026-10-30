/** @file Verifies sparse spreadsheet paging stays bounded and snapshot-bound. */
import { describe, expect, it, vi } from "vitest";
import sharedCellFixture from "../../../../tests/fixtures/rich-artifact/spreadsheet-cell-target.json";
import { SpreadsheetPageController } from "../src";

describe("SpreadsheetPageController", () => {
  it("requests bounded exact-snapshot pages and advances only when more cells exist", async () => {
    const requests: unknown[] = [];
    const client = {
      spreadsheetCells: vi.fn(async (request: unknown) => {
        requests.push(request);
        const offset = (request as { offset: number }).offset;
        return {
          project: "project-1",
          snapshot: "snapshot-1",
          worksheet: "worksheet-1",
          cells: [
            {
              id: `cell-${offset}`,
              row: offset + 1,
              column: 1,
              value: { kind: "number", value: `${offset}` },
            },
          ],
          total: 3,
          hasMore: offset < 2,
        };
      }),
    };
    const controller = new SpreadsheetPageController(
      client as never,
      "project-1",
      "snapshot-1",
      "worksheet-1",
      1,
    );
    const range = { firstRow: 1, firstColumn: 1, lastRow: 3, lastColumn: 1 };
    await controller.read(range);
    await controller.next();
    await controller.next();
    expect(await controller.next()).toBeUndefined();
    expect(requests).toEqual([
      {
        project: "project-1",
        snapshot: "snapshot-1",
        worksheet: "worksheet-1",
        range,
        offset: 0,
        limit: 1,
      },
      {
        project: "project-1",
        snapshot: "snapshot-1",
        worksheet: "worksheet-1",
        range,
        offset: 1,
        limit: 1,
      },
      {
        project: "project-1",
        snapshot: "snapshot-1",
        worksheet: "worksheet-1",
        range,
        offset: 2,
        limit: 1,
      },
    ]);
    expect(controller.currentPage()?.cells[0]?.id).toBe("cell-2");
  });

  it("rejects a page size above the shared API maximum", () => {
    const client = { spreadsheetCells: vi.fn() };
    expect(() => new SpreadsheetPageController(client as never, "p", "s", "w", 501)).toThrow(
      RangeError,
    );
  });

  it("requests only the visible rows and bounded prefetch for a viewport", async () => {
    const spreadsheetCells = vi.fn(async (request: { range: unknown }) => ({
      project: "project-1",
      snapshot: "snapshot-1",
      worksheet: "worksheet-1",
      cells: [],
      total: 0,
      hasMore: false,
      requestedRange: request.range,
    }));
    const controller = new SpreadsheetPageController(
      { spreadsheetCells } as never,
      "project-1",
      "snapshot-1",
      "worksheet-1",
      40,
    );
    await controller.readViewport(
      { firstRow: 1, firstColumn: 2, lastRow: 1000, lastColumn: 8 },
      101,
      30,
      10,
    );
    expect(spreadsheetCells).toHaveBeenCalledWith(
      {
        project: "project-1",
        snapshot: "snapshot-1",
        worksheet: "worksheet-1",
        range: { firstRow: 101, firstColumn: 2, lastRow: 140, lastColumn: 8 },
        offset: 0,
        limit: 40,
      },
      undefined,
    );
  });

  it("rejects a viewport outside worksheet bounds", async () => {
    const controller = new SpreadsheetPageController(
      { spreadsheetCells: vi.fn() } as never,
      "project-1",
      "snapshot-1",
      "worksheet-1",
    );
    await expect(
      controller.readViewport({ firstRow: 1, firstColumn: 1, lastRow: 10, lastColumn: 5 }, 11, 20),
    ).rejects.toThrow("does not overlap");
  });

  it("rejects a page returned for a different exact snapshot", async () => {
    const controller = new SpreadsheetPageController(
      {
        spreadsheetCells: vi.fn(async () => ({
          project: "project-1",
          snapshot: "stale-snapshot",
          worksheet: "worksheet-1",
          cells: [],
          total: 0,
          hasMore: false,
        })),
      } as never,
      "project-1",
      "snapshot-1",
      "worksheet-1",
    );

    await expect(controller.read()).rejects.toThrow("does not match the requested snapshot");
    expect(controller.currentPage()).toBeUndefined();
  });

  it("proposes edits against the controller's exact worksheet snapshot", async () => {
    const proposeSpreadsheetCellEdit = vi.fn(async () => ({
      project: "project-1",
      candidateId: "candidate-1",
      baseSnapshot: "snapshot-1",
      path: "budget.xlsx",
      validation: { status: "pending" as const },
    }));
    const controller = new SpreadsheetPageController(
      {
        spreadsheetCells: vi.fn(async () => ({
          project: "project-1",
          snapshot: "snapshot-1",
          worksheet: sharedCellFixture.worksheet,
          artifactCapability: "lossless_editable" as const,
          cells: [],
          total: 0,
          hasMore: false,
        })),
        proposeSpreadsheetCellEdit,
      } as never,
      "project-1",
      "snapshot-1",
      sharedCellFixture.worksheet,
    );
    await controller.read();
    const result = await controller.proposeCellEdit(
      sharedCellFixture.row,
      sharedCellFixture.column,
      {
        kind: "formula",
        value: { expression: sharedCellFixture.value.expression },
      },
    );
    expect(proposeSpreadsheetCellEdit).toHaveBeenCalledWith(
      {
        project: "project-1",
        snapshot: "snapshot-1",
        worksheet: sharedCellFixture.worksheet,
        row: sharedCellFixture.row,
        column: sharedCellFixture.column,
        value: { kind: "formula", value: { expression: sharedCellFixture.value.expression } },
      },
      undefined,
    );
    expect(result.candidateId).toBe("candidate-1");
  });

  it("rejects candidate requests when the exact page reports read-only fidelity", async () => {
    const proposeSpreadsheetCellEdit = vi.fn();
    const controller = new SpreadsheetPageController(
      {
        spreadsheetCells: vi.fn(async () => ({
          project: "project-1",
          snapshot: "snapshot-1",
          worksheet: "worksheet-1",
          artifactCapability: "read_only_derived" as const,
          artifactDiagnostic: "Workbook fidelity is incomplete.",
          cells: [],
          total: 0,
          hasMore: false,
        })),
        proposeSpreadsheetCellEdit,
      } as never,
      "project-1",
      "snapshot-1",
      "worksheet-1",
    );
    await controller.read();
    await expect(
      controller.proposeCellEdit(1, 1, { kind: "text", value: "changed" }),
    ).rejects.toThrow("Workbook fidelity is incomplete.");
    expect(proposeSpreadsheetCellEdit).not.toHaveBeenCalled();
  });
});
