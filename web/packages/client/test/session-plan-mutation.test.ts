/** @file Verifies semantic plan mutation client requests. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("session plan mutations", () => {
  it("sends semantic plan mutations through the revision-checked operation", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          id: "plan-1",
          revision: 7,
          goal: "work",
          status: "Running",
          root_branch: "branch-1",
          nodes: [],
          dependencies: [],
          history: [],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const plan = await client.mutatePlan({
      session: { id: "session-1" as never },
      plan: "plan-1",
      expectedRevision: 6,
      mutation: {
        AddNextStep: {
          node: {
            key: "next",
            title: "Next",
            purpose: "continue work",
            semanticScope: [],
          },
        },
      },
    });

    expect(plan.revision).toBe(7);
    expect(requests).toEqual([
      {
        operation: "plan_mutation",
        payload: {
          session: { id: "session-1" },
          plan: "plan-1",
          expected_revision: 6,
          mutation: {
            AddNextStep: {
              node: {
                key: "next",
                parent_index: null,
                title: "Next",
                purpose: "continue work",
                expected_result: null,
                semantic_scope: [],
                assignment: null,
              },
            },
          },
        },
      },
    ]);
  });
});
