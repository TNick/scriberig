/** @file Verifies typed running-plan evolution client operations. */

import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("plan evolution client operations", () => {
  it("routes next-step and decomposition through shared application operations", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          id: "plan-1",
          revision: 4,
          goal: "Ship feature",
          status: "Running",
          root_branch: "branch-1",
          nodes: [],
          dependencies: [],
          root_nodes: [],
          status_counts: {},
          gates: [],
          evidence: [],
          summaries: [],
          history: [],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const nextStep = {
      session: { id: "session-1" },
      plan: "plan-1",
      expected_revision: 3,
      title: "Verify release",
      purpose: "Run the release checks",
      expected_result: { kind: "validation", requires_handle: true },
      semantic_scope: ["artifact-1"],
    } as const;
    const decomposition = {
      session: { id: "session-1" },
      plan: "plan-1",
      parent: "node-1",
      expected_revision: 4,
      children: [
        {
          title: "Add coverage",
          purpose: "Cover the edge case",
          expected_result: { kind: "test", requires_handle: false },
          semantic_scope: [],
        },
      ],
      required: true,
    } as const;

    expect((await client.addPlanNextStep(nextStep)).revision).toBe(4);
    expect((await client.decomposePlanNode(decomposition)).revision).toBe(4);
    expect(requests).toEqual([
      { operation: "plan_next_step", payload: nextStep },
      { operation: "plan_decompose", payload: decomposition },
    ]);
  });
});
