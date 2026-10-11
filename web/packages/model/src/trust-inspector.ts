/** @file Provides framework-independent trust inspection state. */

import type { TrustAccessDenied, TrustClient } from "@scriberig/client";
import type { TrustDetails, TrustEvent, TrustSelection, TrustSummary } from "@scriberig/protocol";

/** States exposed by the headless trust inspector. */
export type TrustInspectorState =
  | { status: "idle" }
  | { status: "loading"; selection: TrustSelection }
  | { status: "ready"; selection: TrustSelection; details: TrustDetails }
  | { status: "denied"; selection: TrustSelection; error: TrustAccessDenied }
  | { status: "error"; selection: TrustSelection; error: Error };

/** Coordinates trust data loading and trust-event invalidation.
 *
 * Public members:
 * - `getState`: Reads the current inspector state.
 * - `load`: Loads details for a selection.
 * - `dispose`: Stops event subscriptions.
 *
 * Private members:
 * - `state`: Current state value.
 * - `unsubscribe`: Active event cleanup callback.
 */
export class TrustInspector {
  private state: TrustInspectorState = { status: "idle" };
  private unsubscribe: (() => void) | undefined;

  /**
   * Returns the current immutable inspector state.
   * @returns The current inspector state.
   */
  getState(): TrustInspectorState {
    return this.state;
  }

  /**
   * Loads trust details for a universal semantic selection.
   *
   * @param client Trust data client.
   * @param selection Universal semantic selection.
   * @returns The resulting inspector state.
   */
  async load(client: TrustClient, selection: TrustSelection): Promise<TrustInspectorState> {
    this.unsubscribe?.();
    this.state = { status: "loading", selection };
    this.unsubscribe = client.subscribeTrust(selection, () => this.invalidate());
    try {
      const details = await client.getTrustDetails(selection);
      this.state = { status: "ready", selection, details };
    } catch (error) {
      if (error instanceof Error && error.name === "TrustAccessDenied") {
        this.state = { status: "denied", selection, error: error as TrustAccessDenied };
      } else {
        this.state = {
          status: "error",
          selection,
          error: error instanceof Error ? error : new Error("Trust inspection failed"),
        };
      }
    }
    return this.state;
  }

  /** Stops listening for trust events. */
  dispose(): void {
    this.unsubscribe?.();
    this.unsubscribe = undefined;
  }

  /** Invalidates ready state after a relevant trust event. */
  private invalidate(): void {
    if (this.state.status === "ready") {
      this.state = { status: "loading", selection: this.state.selection };
    }
  }
}

/**
 * Narrows a detailed trust result to its compact summary.
 *
 * @param details Detailed trust result.
 * @returns Its compact summary.
 */
export function trustSummary(details: TrustDetails): TrustSummary {
  return details.summary;
}
