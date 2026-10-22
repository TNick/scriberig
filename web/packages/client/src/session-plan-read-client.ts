/** @file Implements compact and paged semantic plan reads. */
import type {
  SessionPlanDetailsRequest,
  SessionPlanDetailsResponse,
  SessionPlanGraphView,
  SessionPlanViewRequest,
} from "@scriberig/protocol";
import { SemanticClient } from "./semantic-client";
import { normalizePlanDetails } from "./session-plan-details";

/** Provides compact and bounded detail reads for durable plans.
 *
 * Public members: compact graph and paged gate/evidence reads.
 * Private members: inherited semantic transport.
 */
export class SessionPlanReadClient extends SemanticClient {
  /** Reads bounded gate/evidence pages tied to one plan revision or snapshot.
   * @param request Session, exact plan selection, and independent page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Typed gate and evidence records from the observed plan revision.
   */
  getPlanDetails(
    request: SessionPlanDetailsRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlanDetailsResponse> {
    return this.transport
      .request<unknown>(
        "plan_details",
        {
          session: request.session,
          plan: request.plan,
          expected_revision: request.expectedRevision,
          ...(request.snapshot === undefined ? {} : { snapshot: request.snapshot }),
          ...(request.gateOffset === undefined ? {} : { gate_offset: request.gateOffset }),
          ...(request.evidenceOffset === undefined
            ? {}
            : { evidence_offset: request.evidenceOffset }),
          ...(request.limit === undefined ? {} : { limit: request.limit }),
        },
        signal,
      )
      .then(normalizePlanDetails);
  }

  /** Reads a compact plan graph with exact semantic revision references.
   * @param request Session, plan, and optional exact snapshot.
   * @param signal Optional request cancellation signal.
   * @returns Compact graph without history or full detail collections.
   */
  getPlanView(
    request: SessionPlanViewRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlanGraphView> {
    return this.transport.request<SessionPlanGraphView>(
      "plan_view",
      {
        session: request.session,
        plan: request.plan,
        ...(request.snapshot === undefined ? {} : { snapshot: request.snapshot }),
      },
      signal,
    );
  }
}
