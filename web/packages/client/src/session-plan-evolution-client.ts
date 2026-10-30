/** @file Adds shared semantic plan mutation and evolution operations. */

import type {
  SessionPlan,
  SessionPlanDecomposeRequest,
  SessionPlanNextStepRequest,
  SessionPlanSemanticMutation,
} from "@scriberig/protocol";
import { normalizeSessionPlan } from "./session-plan-views";
import { PlanNodeDetailsClient } from "./plan-node-details-client";

/** Client operations that evolve plans through shared application routes.
 *
 * Public members: next-step, decomposition, semantic mutation, and retry calls.
 * Private members: None.
 */
export class SessionPlanEvolutionClient extends PlanNodeDetailsClient {
  /** Appends ordinary work to a running plan at an observed revision.
   * @param request Shared application request for the new work.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative updated plan projection.
   */
  addPlanNextStep(request: SessionPlanNextStepRequest, signal?: AbortSignal): Promise<SessionPlan> {
    return this.transport
      .request<unknown>("plan_next_step", request, signal)
      .then(normalizeSessionPlan);
  }

  /** Decomposes a plan node into structural children through the shared operation.
   * @param request Shared application request for the proposed children.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative updated plan projection.
   */
  decomposePlanNode(
    request: SessionPlanDecomposeRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlan> {
    return this.transport
      .request<unknown>("plan_decompose", request, signal)
      .then(normalizeSessionPlan);
  }

  /** Applies a typed plan summary, evidence, gate, or waiver mutation.
   * @param sessionId Owning durable session.
   * @param planId Stable plan identity.
   * @param expectedRevision Plan mutation revision observed by the caller.
   * @param mutation Shared semantic mutation to apply.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative updated plan projection.
   */
  mutatePlanSemantics(
    sessionId: string,
    planId: string,
    expectedRevision: number,
    mutation: SessionPlanSemanticMutation,
    signal?: AbortSignal,
  ): Promise<SessionPlan> {
    return this.transport
      .request<unknown>(
        "plan_semantic_mutation",
        {
          session: { id: sessionId },
          plan: planId,
          expected_revision: expectedRevision,
          mutation,
        },
        signal,
      )
      .then(normalizeSessionPlan);
  }

  /** Retries a failed node through the shared plan recovery operation.
   * @param sessionId Owning durable session.
   * @param planId Stable plan identity.
   * @param nodeId Failed node identity.
   * @param expectedRevision Plan mutation revision observed by the caller.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative updated plan projection.
   */
  retryPlanNode(
    sessionId: string,
    planId: string,
    nodeId: string,
    expectedRevision: number,
    signal?: AbortSignal,
  ): Promise<SessionPlan> {
    return this.transport
      .request<unknown>(
        "plan_retry",
        {
          session: { id: sessionId },
          plan: planId,
          node: nodeId,
          expected_revision: expectedRevision,
        },
        signal,
      )
      .then(normalizeSessionPlan);
  }
}
