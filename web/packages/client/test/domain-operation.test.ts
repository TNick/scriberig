/** @file Verifies typed semantic operations preserve projection anchor context. */
import { describe, expect, it, vi } from "vitest";
import { ScriberigClient } from "../src";

describe("domain operation client", () => {
  it("carries typed projection anchors with semantic operation identity", async () => {
    const requests: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push([operation, payload]);
        return { operationId: "operation-1", kind: "domain_operation", status: "queued" } as T;
      },
      subscribe: () => () => undefined,
    };
    await new ScriberigClient(transport).domainOperation({
      capability: "spreadsheet.inspect_cell",
      entity: { id: "cell-1" as never },
      snapshot: { id: "snapshot-1" as never },
      arguments: {},
      projectionAnchor: {
        kind: "spreadsheetCell",
        worksheetId: "worksheet-1" as never,
        row: 3,
        column: 2,
      },
    });
    expect(requests).toEqual([
      [
        "command",
        {
          kind: "domain_operation",
          payload: {
            capability: "spreadsheet.inspect_cell",
            entity: { id: "cell-1" },
            snapshot: { id: "snapshot-1" },
            arguments: {},
            projection_anchor: {
              kind: "spreadsheetCell",
              worksheetId: "worksheet-1",
              row: 3,
              column: 2,
            },
          },
        },
      ],
    ]);
  });

  it("rejects an oversized opaque anchor before transport", async () => {
    const request = vi.fn();
    const client = new ScriberigClient({
      request,
      subscribe: () => () => undefined,
    } as never);
    expect(() =>
      client.domainOperation({
        capability: "domain.operation",
        entity: { id: "entity-1" as never },
        snapshot: { id: "snapshot-1" as never },
        arguments: {},
        projectionAnchor: {
          kind: "opaque",
          adapter: "adapter",
          anchorKind: "shape",
          value: "x".repeat(4097),
        },
      }),
    ).toThrow("opaque anchor values exceed descriptor bounds");
    expect(() =>
      client.domainOperation({
        capability: "domain.operation",
        entity: { id: "entity-1" as never },
        snapshot: { id: "snapshot-1" as never },
        arguments: {},
        projectionAnchor: {
          kind: "spreadsheetCell",
          worksheetId: "w".repeat(129) as never,
          row: 1,
          column: 1,
        },
      }),
    ).toThrow("spreadsheet cell anchor coordinates are invalid");
    expect(request).not.toHaveBeenCalled();
  });
});
