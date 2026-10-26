/** @file Verifies explicit recovery operations for preserved execution choices. */
import { describe, expect, it } from "vitest";
import type { SessionId } from "@scriberig/protocol";
import { ScriberigClient } from "../src";

describe("legacy execution recovery client", () => {
  it("reads and explicitly maps preserved legacy execution choices", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return (
          operation === "legacy_execution_records"
            ? [
                {
                  path: "/plans/0/nodes/0/assignment",
                  value: { model: "retired-name" },
                  target: { plan_id: "plan-1", node_id: "node-1", claim: false },
                },
              ]
            : operation === "legacy_plan_assignment_mapping"
              ? { id: "plan-1", revision: 5, nodes: [], dependencies: [] }
              : { revision: 4, provider_id: "provider-a", model_id: "model-a", history: [] }
        ) as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const sessionId = "session-a" as SessionId;
    const records = await client.getLegacyExecutionRecords(sessionId);
    expect(records).toEqual([
      {
        path: "/plans/0/nodes/0/assignment",
        value: { model: "retired-name" },
        target: { planId: "plan-1", nodeId: "node-1", claim: false },
      },
    ]);
    await client.mapLegacyExecutionSelection(
      sessionId,
      3,
      records[0]!.path,
      "provider-a",
      "model-a",
    );
    const plan = await client.mapLegacyPlanAssignment(
      sessionId,
      records[0]!.path,
      4,
      "provider-a",
      "model-a",
    );
    expect(plan.revision).toBe(5);
    expect(requests).toEqual([
      {
        operation: "legacy_execution_records",
        payload: { session: { id: "session-a" } },
      },
      {
        operation: "legacy_execution_mapping",
        payload: {
          session: { id: "session-a" },
          expected_revision: 3,
          legacy_path: "/plans/0/nodes/0/assignment",
          provider_id: "provider-a",
          model_id: "model-a",
        },
      },
      {
        operation: "legacy_plan_assignment_mapping",
        payload: {
          session: { id: "session-a" },
          legacy_path: "/plans/0/nodes/0/assignment",
          expected_plan_revision: 4,
          provider_id: "provider-a",
          model_id: "model-a",
        },
      },
    ]);
  });
});
