/** @file Verifies remote spreadsheet worksheet rename candidate requests. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("spreadsheet worksheet rename", () => {
  /** Sends one rename request and returns deterministic candidate metadata. */
  it("uses the shared candidate operation", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          candidate_id: "candidate-rename",
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
      new_name: "Plan",
    };
    const candidate = await new ScriberigClient(transport).proposeSpreadsheetRename(request);
    expect(candidate.candidateId).toBe("candidate-rename");
    expect(candidate.baseSnapshot).toBe("snapshot-1");
    expect(calls).toEqual([["spreadsheet_rename", request]]);
  });
});
