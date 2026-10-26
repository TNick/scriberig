/** @file Checks web plan normalization against the shared Rust/server fixture. */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { ScriberigClient } from "../../packages/client/src";

const serverPlanView = JSON.parse(
  readFileSync(resolve(process.cwd(), "../tests/conformance/fixtures/plan-view.json"), "utf8"),
) as Record<string, unknown>;

it("normalizes the shared plan graph without recalculating semantics", async () => {
  const requests: { operation: string; payload: unknown }[] = [];
  const client = new ScriberigClient({
    request: async <T>(operation: string, payload: unknown): Promise<T> => {
      requests.push({ operation, payload });
      return serverPlanView as T;
    },
    subscribe: () => () => undefined,
  });
  const graph = await client.getPlanView({
    session: { id: "ent_72727272727272727272727272727272" as never },
    plan: "ent_c1000000000000000000000000000005",
    snapshot: "snp_e18b09aa5b17c8326b63ebf4835f1e791d46c4e6ac765daee061af44bf22e7f3",
  });

  expect(graph.plan).toMatchObject({
    id: "ent_c1000000000000000000000000000005",
    revision: "rev_f5997db286cf72901c8d61c54dbcf1cfb22c75a6b31632b5bf45968cd57c87d0",
    mutationRevision: 4,
    status: "Planned",
    rootNodes: ["ent_c2000000000000000000000000000006"],
    nodeCount: 1,
    statusCounts: { ready: 1 },
  });
  expect(graph.nodes).toMatchObject([
    {
      id: "ent_c2000000000000000000000000000006",
      directStatus: "Ready",
      effectiveStatus: "Ready",
      dependencyBlockers: [],
      entryGates: { pending: 0, satisfied: 1, failed: 0, waived: 0 },
      evidenceCount: 1,
      canRetry: false,
    },
  ]);
  expect(requests).toEqual([
    {
      operation: "plan_view",
      payload: {
        session: { id: "ent_72727272727272727272727272727272" },
        plan: "ent_c1000000000000000000000000000005",
        snapshot: "snp_e18b09aa5b17c8326b63ebf4835f1e791d46c4e6ac765daee061af44bf22e7f3",
      },
    },
  ]);
});
