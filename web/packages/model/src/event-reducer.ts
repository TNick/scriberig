/** @file Applies protocol events to model-owned state transitions. */
import type { ProtocolEvent } from "@scriberig/protocol";
import { advanceSnapshot, type SnapshotState } from "./snapshot-state";
import type { OperationStore } from "./operation-store";
/** Minimal mutable state owned by the event reducer. */
export interface ModelEventState {
  snapshots: SnapshotState;
  disconnected: boolean;
  changedBranches: readonly string[];
  changedSessions: readonly string[];
  lastEvent?: ProtocolEvent;
}
/** Applies one event without mutating immutable revision caches. @param state Current state. @param event Incoming event. @param operations Operation store. @returns Updated state. */
export function reduceEvent(
  state: ModelEventState,
  event: ProtocolEvent,
  operations: OperationStore,
): ModelEventState {
  const observed = { ...state, lastEvent: event };
  switch (event.kind) {
    case "snapshotAdvanced":
      return { ...observed, snapshots: advanceSnapshot(state.snapshots, event.snapshotId) };
    case "operationProgress":
      operations.progress(event.operationId, event.progress);
      return observed;
    case "operationStarted":
      operations.start(event.operationId);
      return observed;
    case "operationCompleted":
      operations.complete(event.operationId);
      return observed;
    case "branchChanged":
      return { ...observed, changedBranches: [...state.changedBranches, event.branchId] };
    case "sessionChanged":
      return { ...observed, changedSessions: [...state.changedSessions, event.sessionId] };
    case "operationFailed":
      operations.fail(event.operationId, event.error);
      return observed;
    case "operationCancelled":
      operations.cancel(event.operationId);
      return observed;
    case "agentOutput":
      operations.output(event.operationId, event.text);
      return observed;
    default:
      return observed;
  }
}
