/** @file Applies protocol events to model-owned state transitions. */
import type { ProtocolEvent } from "@scriberig/protocol";
import { advanceSnapshot, type SnapshotState } from "./snapshot-state";
import type { OperationStore } from "./operation-store";
/** Minimal mutable state owned by the event reducer. */
export interface ModelEventState {
  snapshots: SnapshotState;
  disconnected: boolean;
}
/** Applies one event without mutating immutable revision caches. @param state Current state. @param event Incoming event. @param operations Operation store. @returns Updated state. */
export function reduceEvent(
  state: ModelEventState,
  event: ProtocolEvent,
  operations: OperationStore,
): ModelEventState {
  switch (event.kind) {
    case "snapshotAdvanced":
      return { ...state, snapshots: advanceSnapshot(state.snapshots, event.snapshotId) };
    case "operationProgress":
      operations.progress(event.operationId, event.progress);
      return state;
    default:
      return state;
  }
}
