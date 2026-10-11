/** @file Binds headless migration review state to React. */
import type { MigrationReviewState } from "@scriberig/editor-core";
/** Host-owned actions for a migration review surface. */
export interface MigrationReviewActions {
  /** Applies the validated proposal. */
  apply: () => void;
  /** Rejects the proposal without changing canonical state. */
  reject: () => void;
}

/** Exposes migration review state and host-supplied actions.
 * @param state Review state.
 * @param actions Actions owned by the application/runtime host.
 * @returns Review state and the supplied host actions.
 */
export function useMigrationReview(
  state: MigrationReviewState,
  actions: MigrationReviewActions,
): { state: MigrationReviewState } & MigrationReviewActions {
  return { state, ...actions };
}
