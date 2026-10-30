/** @file Adds bounded plan attempt reads to the semantic client base. */
import type {
  SessionPlanAttemptHistoryPage,
  SessionPlanAttemptHistoryRequest,
} from "@scriberig/protocol";
import { normalizePlanAttemptHistory } from "./session-plan-attempt-views";
import { SemanticClient } from "./semantic-client";

/** Client operations for exact-snapshot plan attempt history.
 *
 * Public members: getPlanAttemptHistory reads a bounded node attempt page.
 * Private members: None.
 */
export class PlanAttemptHistoryClient extends SemanticClient {
  /** Reads one bounded page of a node's attempts at an exact snapshot.
   * @param request Session, plan, node, snapshot, and page range.
   * @param signal Optional request cancellation signal.
   * @returns The exact-snapshot attempt page.
   */
  getPlanAttemptHistory(
    request: SessionPlanAttemptHistoryRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlanAttemptHistoryPage> {
    return this.transport
      .request("plan_attempt_history", request, signal)
      .then(normalizePlanAttemptHistory);
  }
}
