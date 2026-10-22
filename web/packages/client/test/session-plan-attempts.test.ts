/** @file Verifies bounded plan-attempt client reads. */

import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("plan attempt client", () => {
  it("requests a bounded page and normalizes safe attempt metadata", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          plan: "plan-1",
          node: "node-1",
          total: 3,
          offset: 1,
          next_offset: 2,
          attempts: [
            {
              branch: "branch-2",
              claim_generation: 1,
              claim: "claim-1",
              status: "Failed",
              result: null,
              failure: {
                kind: "Failure",
                reference: { entity: "failure-1" },
                attributes: { reason: "tool failure" },
              },
            },
          ],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const page = await client.getPlanAttempts({
      session: { id: "session-1" as never },
      plan: "plan-1",
      node: "node-1",
      offset: 1,
      limit: 1,
    });

    expect(page.attempts[0]).toMatchObject({
      branch: "branch-2",
      claimGeneration: 1,
      status: "Failed",
      failure: { kind: "Failure" },
    });
    expect(page.nextOffset).toBe(2);
    expect(requests).toEqual([
      {
        operation: "plan_attempts",
        payload: {
          session: { id: "session-1" },
          plan: "plan-1",
          node: "node-1",
          offset: 1,
          limit: 1,
        },
      },
    ]);
  });
});
