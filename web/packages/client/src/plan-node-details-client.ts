/** @file Adds bounded gate and evidence reads to the session client. */

import type {
  SessionPlanNodeDetailsPage,
  SessionPlanNodeDetailsRequest,
} from "@scriberig/protocol";
import { normalizePlanNodeDetails } from "./session-plan-semantic-views";
import { PlanAttemptHistoryClient } from "./plan-attempt-history-client";

/** Client operations for exact-snapshot gate and evidence detail.
 *
 * Public members: getPlanNodeDetails reads one bounded node detail page.
 * Private members: None.
 */
export class PlanNodeDetailsClient extends PlanAttemptHistoryClient {
  /** Reads one bounded page of a node's gates and evidence.
   * @param request Session, plan, node, snapshot, and page range.
   * @param signal Optional request cancellation signal.
   * @returns The exact-snapshot node detail page.
   */
  getPlanNodeDetails(
    request: SessionPlanNodeDetailsRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlanNodeDetailsPage> {
    return this.transport
      .request("plan_node_details", request, signal)
      .then(normalizePlanNodeDetails);
  }
}
