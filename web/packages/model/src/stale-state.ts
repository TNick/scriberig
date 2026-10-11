/** @file Defines structured stale-state predicates. */
import type { SnapshotId } from "@scriberig/protocol";
/** Describes whether a view is historical or stale. */
export interface StaleState {
  historical: boolean;
  stale: boolean;
}
/** Computes snapshot freshness. @param viewed Viewed snapshot. @param current Current snapshot. @returns Structured freshness. */
export function staleState(
  viewed: SnapshotId | undefined,
  current: SnapshotId | undefined,
): StaleState {
  return { historical: viewed !== undefined && viewed !== current, stale: viewed !== current };
}
