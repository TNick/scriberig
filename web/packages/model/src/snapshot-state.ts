/** @file Tracks current, viewed, candidate, and stale snapshot state. */
import type { SnapshotId } from "@scriberig/protocol";
/** Immutable snapshot selections known to the client model. */
export interface SnapshotState {
  current?: SnapshotId;
  viewed?: SnapshotId;
  candidate?: SnapshotId;
  stale: boolean;
}
/** Creates an empty snapshot state. @returns Initial state. */
export function createSnapshotState(): SnapshotState {
  return { stale: false };
}
/** Advances live state without changing historical views. @param state Existing state. @param current New snapshot. @returns Updated state. */
export function advanceSnapshot(state: SnapshotState, current: SnapshotId): SnapshotState {
  return { ...state, current, stale: state.viewed !== undefined && state.viewed !== current };
}
