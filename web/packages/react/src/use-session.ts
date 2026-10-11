/** @file Exposes semantic session summaries to React consumers. */
import type { BranchSummary, SessionSummary } from "@scriberig/protocol";
/** Session view input. */
export interface SessionView {
  session?: SessionSummary;
  branch?: BranchSummary;
  loading: boolean;
  error?: unknown;
}
/** Exposes a session summary without reducing it to chat messages.
 * @param view Session and branch data supplied by the model boundary.
 * @returns Semantic session view.
 */
export function useSession(view: SessionView = { loading: false }): SessionView {
  return view;
}
