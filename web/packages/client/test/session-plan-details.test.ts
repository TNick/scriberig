/** @file Verifies bounded plan gate and evidence client reads. */

import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

describe("plan details client", () => {
  it("requests exact-revision pages and normalizes their offsets", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          plan: "plan-1",
          revision: 8,
          snapshot: "snapshot-8",
          gate_total: 3,
          gate_offset: 1,
          gate_next_offset: 2,
          gates: [
            {
              id: "gate-2",
              owner: "node-1",
              phase: "Entry",
              requirement: { EvidencePresent: { kind: "Artifact" } },
            },
          ],
          evidence_total: 4,
          evidence_offset: 2,
          evidence_next_offset: 3,
          evidence: [
            {
              id: "evidence-3",
              owner: { Node: "node-1" },
              role: "Supports",
              evidence: {
                kind: "Artifact",
                reference: { entity: "artifact-1" },
                attributes: {},
              },
            },
          ],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const details = await client.getPlanDetails({
      session: { id: "session-1" as never },
      plan: "plan-1",
      expectedRevision: 8,
      gateOffset: 1,
      evidenceOffset: 2,
      limit: 1,
    });

    expect(details).toMatchObject({
      revision: 8,
      snapshot: "snapshot-8",
      gateTotal: 3,
      gateNextOffset: 2,
      evidenceTotal: 4,
      evidenceNextOffset: 3,
      gates: [{ id: "gate-2", phase: "Entry" }],
      evidence: [{ id: "evidence-3", role: "Supports" }],
    });
    expect(requests).toEqual([
      {
        operation: "plan_details",
        payload: {
          session: { id: "session-1" },
          plan: "plan-1",
          expected_revision: 8,
          gate_offset: 1,
          evidence_offset: 2,
          limit: 1,
        },
      },
    ]);
  });
});
