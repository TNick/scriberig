/** @file Binds headless migration review state to React. */
import type { MigrationReviewState } from "@scriberig/editor-core";
/** Exposes migration review state and host-supplied actions.
 * @param state Review state.
 * @returns Review state and no-op-safe actions for the host controller.
 */
export function useMigrationReview(state: MigrationReviewState) {
  return { state, apply: () => undefined, reject: () => undefined };
}
