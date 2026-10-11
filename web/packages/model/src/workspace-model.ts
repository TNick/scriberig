/** @file Composes framework-independent semantic client model state. */
import type { ProtocolEvent, SnapshotId } from "@scriberig/protocol";
import { RevisionCache } from "./revision-cache";
import { createSnapshotState, type SnapshotState } from "./snapshot-state";
import { OperationStore } from "./operation-store";
import { reduceEvent, type ModelEventState } from "./event-reducer";
import { Subscription } from "./subscription";
/** Client-side semantic state coordinated by immutable server identities.
 *
 * Public members: `revisions`, `operations`, `subscribe`, `apply`,
 * `setDisconnected`, `view`, and `getState`.
 * Private members: the event state and change subscription.
 */
export class WorkspaceModel {
  /** Immutable revision cache. */
  readonly revisions = new RevisionCache();
  /** Durable operation records. */
  readonly operations = new OperationStore();
  private state: ModelEventState = { snapshots: createSnapshotState(), disconnected: false };
  private readonly changes = new Subscription<ModelEventState>();
  /** Subscribes to consistent state transitions. @param listener State listener. @returns Unsubscribe function. */
  subscribe(listener: (state: ModelEventState) => void): () => void {
    return this.changes.subscribe(listener);
  }
  /** Applies one protocol event. @param event Incoming event. */
  apply(event: ProtocolEvent): void {
    this.state = reduceEvent(this.state, event, this.operations);
    this.changes.notify(this.state);
  }
  /** Marks the live connection state. @param disconnected Whether disconnected. */
  setDisconnected(disconnected: boolean): void {
    this.state = { ...this.state, disconnected };
    this.changes.notify(this.state);
  }
  /** Selects a historical snapshot without advancing it on live events. @param snapshot Snapshot to view. */
  view(snapshot: SnapshotId): void {
    this.state = {
      ...this.state,
      snapshots: {
        ...this.state.snapshots,
        viewed: snapshot,
        stale: this.state.snapshots.current !== snapshot,
      },
    };
    this.changes.notify(this.state);
  }
  /** Returns current model state. @returns Consistent state snapshot. */
  getState(): ModelEventState {
    return this.state;
  }
}
