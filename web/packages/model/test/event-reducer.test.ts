/** @file Tests preservation of protocol events for model consumers. */

import { describe, expect, it } from "vitest";
import type { OperationId, ProtocolEvent } from "@scriberig/protocol";
import { OperationStore, reduceEvent, createSnapshotState } from "../src";

describe("reduceEvent", () => {
  it("retains non-snapshot lifecycle events for subscribed views", () => {
    const event: ProtocolEvent = { kind: "branchChanged", branchId: "branch-1" };
    const state = reduceEvent(
      {
        snapshots: createSnapshotState(),
        disconnected: false,
        changedBranches: [],
        changedSessions: [],
      },
      event,
      new OperationStore(),
    );

    expect(state.lastEvent).toEqual(event);
  });

  it("records branch and session changes for model consumers", () => {
    const operations = new OperationStore();
    const state = reduceEvent(
      reduceEvent(
        {
          snapshots: createSnapshotState(),
          disconnected: false,
          changedBranches: [],
          changedSessions: [],
        },
        { kind: "branchChanged", branchId: "branch-1" },
        operations,
      ),
      { kind: "sessionChanged", sessionId: "session-1" },
      operations,
    );

    expect(state.changedBranches).toEqual(["branch-1"]);
    expect(state.changedSessions).toEqual(["session-1"]);
  });

  it("updates operation lifecycle state from protocol events", () => {
    const operations = new OperationStore();
    const base = {
      snapshots: createSnapshotState(),
      disconnected: false,
      changedBranches: [],
      changedSessions: [],
    };

    reduceEvent(
      base,
      { kind: "operationProgress", operationId: "op-1" as OperationId, progress: 0.5 },
      operations,
    );
    expect(operations.get("op-1" as OperationId)).toMatchObject({
      operationId: "op-1",
      status: "running",
      progress: 0.5,
    });

    reduceEvent(
      base,
      { kind: "operationFailed", operationId: "op-1" as OperationId, error: "bad input" },
      operations,
    );
    expect(operations.get("op-1" as OperationId)).toMatchObject({
      status: "failed",
      error: "bad input",
    });

    reduceEvent(
      base,
      { kind: "operationCancelled", operationId: "op-1" as OperationId },
      operations,
    );
    expect(operations.get("op-1" as OperationId)).toMatchObject({ status: "cancelled" });
  });

  it("retains agent output on the corresponding operation", () => {
    const operations = new OperationStore();

    reduceEvent(
      {
        snapshots: createSnapshotState(),
        disconnected: false,
        changedBranches: [],
        changedSessions: [],
      },
      { kind: "agentOutput", operationId: "op-2" as OperationId, text: "result" },
      operations,
    );

    expect(operations.get("op-2" as OperationId)).toMatchObject({
      output: "result",
      status: "running",
    });
  });
});
