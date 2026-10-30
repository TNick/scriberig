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
    const assignment = {
      session: { id: "session-1" },
      plan: "plan-1",
      node: "node-1",
      expected_revision: 5,
      assignment: {
        provider_id: "provider-1",
        model_id: "model-1",
        thinking_level: "deep",
      },
    } as const;
    const creation = {
      session: { id: "session-1" },
      anchor: { BranchHead: { branch: "branch-1" as never } },
      goal: "Ship feature",
      nodes: [
        {
          key: "first",
          parent_index: null,
          title: "Implement",
          purpose: "Write the feature",
          expected_result: null,
          semantic_scope: [],
          assignment: null,
        },
      ],
      dependencies: [],
    } as const;
    const replan = {
      session: { id: "session-1" },
      plan: "plan-1",
      expected_revision: 7,
      additions: [
        {
          key: "repair",
          parent_index: null,
          title: "Repair",
          purpose: "Fix failed work",
          expected_result: null,
          semantic_scope: [],
          assignment: null,
        },
      ],
      dependencies: [[{ Existing: { id: "node-0" } }, { Proposed: { key: "repair" } }]],
      removed_dependencies: [["node-0", "node-1"]],
      superseded: [["node-1", "repair"]],
      assignment_changes: [["node-2", null]],
      preserved_terminal_nodes: ["node-0"],
      reason: "Replace failed work",
    } as const;

    expect((await client.createPlan(creation)).revision).toBe(4);
    expect((await client.replan(replan)).revision).toBe(4);
    expect((await client.addPlanNextStep(nextStep)).revision).toBe(4);
    expect((await client.decomposePlanNode(decomposition)).revision).toBe(4);
    expect((await client.setPlanAssignment(assignment)).revision).toBe(4);
    expect((await client.mutatePlanLifecycle("session-1", "plan-1", 6, "Start")).revision).toBe(4);
    expect(requests).toEqual([
      { operation: "plan_create", payload: creation },
      { operation: "plan_replan", payload: replan },
      { operation: "plan_next_step", payload: nextStep },
      { operation: "plan_decompose", payload: decomposition },
      { operation: "plan_assignment", payload: assignment },
      {
        operation: "plan_lifecycle",
        payload: {
          session: { id: "session-1" },
          plan: "plan-1",
          expected_revision: 6,
          action: "Start",
        },
      },
    ]);
  });
});
