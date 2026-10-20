/** @file Composes framework-independent semantic client model state. */
import type { ProtocolEvent, SnapshotId } from "@scriberig/protocol";
import { RevisionCache } from "./revision-cache";
import { createSnapshotState, type SnapshotState } from "./snapshot-state";
import { OperationStore } from "./operation-store";
import { reduceEvent, type ModelEventState } from "./event-reducer";
import { Subscription } from "./subscription";
import {
  createAgentWorkspaceState,
  reduceAgentEvent,
  storeAgentOperation,
  storeAgentSession,
  markAgentWorkspaceStale,
  type AgentWorkspaceState,
  type AgentPresentationPreferences,
  type AgentPresentationPreferenceLayer,
} from "./agent-workspace";
import type { ModelLibraryResponse } from "@scriberig/protocol";
import {
  storeAgentModelLibrary,
  storeAgentPresentation,
  storeSharedAgentPresentation,
} from "./agent-workspace";
import type { AgentEvent } from "@scriberig/protocol";
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
  /** Durable branch state and transient agent stream buffers. */
  private agentState: AgentWorkspaceState = createAgentWorkspaceState();
  private state: ModelEventState = {
    snapshots: createSnapshotState(),
    disconnected: false,
    changedBranches: [],
    changedSessions: [],
  };
  private readonly changes = new Subscription<ModelEventState>();
  private readonly agentChanges = new Subscription<AgentWorkspaceState>();
  /** Subscribes to consistent state transitions. @param listener State listener. @returns Unsubscribe function. */
  subscribe(listener: (state: ModelEventState) => void): () => void {
    return this.changes.subscribe(listener);
  }
  /** Subscribes to branch-aware agent state changes. @param listener Agent state listener. @returns Unsubscribe function. */
  subscribeAgent(listener: (state: AgentWorkspaceState) => void): () => void {
    return this.agentChanges.subscribe(listener);
  }
  /** Applies one protocol event. @param event Incoming event. */
  apply(event: ProtocolEvent): void {
    this.state = reduceEvent(this.state, event, this.operations);
    this.changes.notify(this.state);
  }
  /** Applies an ordered agent event to the branch-aware model state. @param event Agent event. */
  applyAgentEvent(event: AgentEvent): void {
    this.agentState = reduceAgentEvent(this.agentState, event);
    this.agentChanges.notify(this.agentState);
  }
  /** Stores an authoritative agent operation beside branch state.
   * @param operation Operation record.
   */
  setAgentOperation(operation: import("@scriberig/protocol").Operation): void {
    this.agentState = storeAgentOperation(this.agentState, operation);
    this.agentChanges.notify(this.agentState);
  }
  /** Stores an authoritative session branch tree. @param session Session response. */
  setAgentSession(session: import("@scriberig/protocol").SessionReadResponse): void {
    this.agentState = storeAgentSession(this.agentState, session);
    this.agentChanges.notify(this.agentState);
  }

  /** Stores the safe public model library for reusable agent controls.
   * @param library Shared upstream-derived model descriptors.
   */
  setAgentModelLibrary(library: ModelLibraryResponse): void {
    this.agentState = storeAgentModelLibrary(this.agentState, library);
    this.agentChanges.notify(this.agentState);
  }

  /** Updates local display choices without creating session history.
   * @param presentation Client-local activity presentation choices.
   */
  setAgentPresentation(presentation: AgentPresentationPreferenceLayer): void {
    this.agentState = storeAgentPresentation(this.agentState, presentation);
    this.agentChanges.notify(this.agentState);
  }
  /** Applies shared defaults beneath any client-local presentation overrides.
   * @param presentation Shared preferences for the authenticated principal.
   */
  setSharedAgentPresentation(presentation: AgentPresentationPreferenceLayer): void {
    this.agentState = storeSharedAgentPresentation(this.agentState, presentation);
    this.agentChanges.notify(this.agentState);
  }
  /** Marks agent state stale until a reconnect refresh completes. */
  markAgentWorkspaceStale(): void {
    this.agentState = markAgentWorkspaceStale(this.agentState);
    this.agentChanges.notify(this.agentState);
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
  /** Returns the current branch-aware agent state. @returns Agent workspace state. */
  getAgentState(): AgentWorkspaceState {
    return this.agentState;
  }
}
