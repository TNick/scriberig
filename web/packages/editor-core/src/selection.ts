/** @file Defines framework-independent semantic selections and control. */
import type { EntityId, RevisionId, SnapshotId } from "@scriberig/protocol";
/** Semantic selection independent of projection coordinates. */
export interface Selection {
  entityId: EntityId;
  snapshotId: SnapshotId;
  revisionId?: RevisionId;
  relationPath?: readonly EntityId[];
  projection?: { key: string; location?: string } | undefined;
}
/** Compares semantic identity while ignoring projection location. @param left First selection. @param right Second selection. @returns Whether semantic references match. */
export function sameSelection(left: Selection | undefined, right: Selection | undefined): boolean {
  return (
    left?.entityId === right?.entityId &&
    left?.snapshotId === right?.snapshotId &&
    left?.revisionId === right?.revisionId
  );
}
/** Controls active selection with subscriptions.
 *
 * Public members: `get`, `set`, `clear`, `subscribe`, and `updateProjection`.
 * Private members: current selection and listener set.
 */
export class SelectionController {
  private current: Selection | undefined;
  private readonly listeners = new Set<(selection?: Selection) => void>();
  /** Reads active selection. @returns Active selection. */
  get(): Selection | undefined {
    return this.current;
  }
  /** Sets selection. @param selection New selection. */
  set(selection: Selection): void {
    this.current = selection;
    for (const listener of this.listeners) listener(this.current);
  }
  /** Clears selection. */
  clear(): void {
    this.current = undefined;
    for (const listener of this.listeners) listener(undefined);
  }
  /** Subscribes to changes. @param listener Selection listener. @returns Unsubscribe function. */
  subscribe(listener: (selection?: Selection) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  /** Updates projection coordinates without changing semantic identity. @param projection Projection metadata. */
  updateProjection(projection: Selection["projection"]): void {
    if (this.current) {
      const next = { ...this.current };
      if (projection === undefined) delete next.projection;
      else next.projection = projection;
      this.set(next);
    }
  }
}
