/** @file Tests preservation of protocol events for model consumers. */

import { describe, expect, it } from "vitest";
import type { ProtocolEvent } from "@scriberig/protocol";
import { OperationStore, reduceEvent, createSnapshotState } from "../src";

describe("reduceEvent", () => {
  it("retains non-snapshot lifecycle events for subscribed views", () => {
    const event: ProtocolEvent = { kind: "branchChanged", branchId: "branch-1" };
    const state = reduceEvent(
      { snapshots: createSnapshotState(), disconnected: false },
      event,
      new OperationStore(),
    );

    expect(state.lastEvent).toEqual(event);
  });
});
