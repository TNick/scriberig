/** @file Tests headless trust state and event invalidation. */

import { describe, expect, it } from "vitest";
import { TrustAccessDenied, type TrustClient } from "@scriberig/client";
import { TrustInspector } from "@scriberig/model";
import type { TrustDetails, TrustEvent, TrustSelection } from "@scriberig/protocol";

const selection = { entityId: "entity-1", snapshotId: "snapshot-1" } as TrustSelection;
const details: TrustDetails = {
  summary: { source: "model_generated", validation: "current", restricted: false },
  provenance: [],
  diagnostics: [],
};

/** Creates a deterministic client double for state tests. */
/**
 * @param listener Event callback capture.
 * @returns A deterministic trust client.
 * @throws Never; the test client does not throw during construction.
 */
function clientFor(listener: { current?: (event: TrustEvent) => void }): TrustClient {
  return {
    getTrustSummary: async () => details.summary,
    getTrustDetails: async () => details,
    subscribeTrust: (_selection, callback) => {
      listener.current = callback;
      return () => delete listener.current;
    },
  };
}

describe("TrustInspector", () => {
  it("loads a selection and invalidates on trust events", async () => {
    const listener: { current?: (event: TrustEvent) => void } = {};
    const inspector = new TrustInspector();
    const client = clientFor(listener);
    expect((await inspector.load(client, selection)).status).toBe("ready");
    listener.current?.({ type: "validation_updated", selection });
    expect(inspector.getState().status).toBe("loading");
    inspector.dispose();
  });

  it("preserves stale validation and structured denial states", async () => {
    const inspector = new TrustInspector();
    const denied: TrustClient = {
      getTrustSummary: async () => ({ ...details.summary, validation: "stale" }),
      getTrustDetails: async () => {
        throw new TrustAccessDenied({
          action: "inspect",
          scope: "entity",
          reason: "restricted",
          alternatives: ["query"],
          approvalPossible: false,
        });
      },
      subscribeTrust: () => () => undefined,
    };
    expect((await inspector.load(denied, selection)).status).toBe("denied");
    expect((await denied.getTrustSummary(selection)).validation).toBe("stale");
  });
});
