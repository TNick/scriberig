/** @file Binds React views to exact model snapshot state. */
import { useSyncExternalStore } from "react";
import type { SnapshotId } from "@scriberig/protocol";
import { useScriberig } from "./provider";

/** Snapshot view classification exposed to components. */
export type SnapshotViewKind = "current" | "historical" | "stale" | "candidate";
/** Snapshot state observed by a React component. */
export interface SnapshotView {
  current: SnapshotId | undefined;
  viewed: SnapshotId | undefined;
  kind: SnapshotViewKind;
}

/** Subscribes to model snapshot state without duplicating it in React state.
 * @param requested Optional exact snapshot to view.
 * @returns Current and viewed snapshot classification.
 */
export function useSnapshot(requested?: SnapshotId): SnapshotView {
  const { model } = useScriberig();
  const state = useSyncExternalStore(
    model.subscribe.bind(model),
    model.getState.bind(model),
    model.getState.bind(model),
  );
  const viewed = requested ?? state.snapshots.viewed ?? state.snapshots.current;
  const current = state.snapshots.current;
  const kind: SnapshotViewKind =
    requested && requested !== current ? "historical" : state.snapshots.stale ? "stale" : "current";
  return { current, viewed, kind };
}
