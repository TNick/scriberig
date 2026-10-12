/** @file Verifies authoritative session and branch projections. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("session client projections", () => {
  it("normalizes Rust session reads and derives branch views", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          session: { id: "session-1" },
          branches: [
            {
              id: "root",
              title: "Root",
              status: "Completed",
              children: ["child"],
              turns: [{ id: "turn-1", ordinal: 1, has_human_input: true, has_agent_output: false }],
            },
            {
              id: "child",
              parent: "root",
              title: "Child",
              status: "Completed",
              children: [],
              turns: [],
              result: { id: "result-1", references: ["migration-1"] },
            },
          ],
        }) as T,
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    const session = await client.getSession("session-1" as never);
    expect(session.branches[0]?.turns[0]?.hasHumanInput).toBe(true);
    expect(await client.getBranch("session-1" as never, "child" as never)).toMatchObject({
      branchId: "child",
      status: "completed",
    });
    expect(await client.listBranchChildren("session-1" as never, "root" as never)).toMatchObject({
      items: [{ branchId: "child" }],
    });
    expect(await client.listBranchActivity("session-1" as never, "root" as never)).toMatchObject({
      items: [{ id: "turn-1", kind: "human" }],
    });
    expect(await client.getBranchResult("session-1" as never, "result-1")).toEqual({
      branchId: "child",
      resultId: "result-1",
      handles: ["migration-1"],
    });
  });
});
