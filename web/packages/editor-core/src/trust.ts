/** @file Defines universal selection helpers for trust inspection. */

import type { TrustSelection } from "@scriberig/protocol";

/**
 * Returns whether two trust selections identify the same exact object.
 *
 * @param left First selection.
 * @param right Second selection.
 * @returns Whether both selections identify the same object.
 */
export function sameTrustSelection(left: TrustSelection, right: TrustSelection): boolean {
  return (
    left.entityId === right.entityId &&
    left.snapshotId === right.snapshotId &&
    left.revisionId === right.revisionId
  );
}
