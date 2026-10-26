/** @file Implements explicit recovery operations for preserved legacy choices. */
import type {
  LegacyExecutionRecordView,
  LegacyExecutionMappingRequest,
  LegacyPlanAssignmentMappingRequest,
  LegacyExecutionRecordsRequest,
  SessionExecutionSelection,
  SessionPlan,
  SessionId,
} from "@scriberig/protocol";
import { normalizeExecutionSelection, normalizeLegacyExecutionRecords } from "./model-views";
import { normalizeSessionPlan } from "./session-plan-views";
import { SessionPlanReadClient } from "./session-plan-read-client";

/** Adds operator-directed legacy execution recovery to durable session clients.
 *
 * Public members: legacy record read and explicit selection mapping.
 * Private members: inherited semantic transport.
 */
export class LegacyExecutionRecoveryClient extends SessionPlanReadClient {
  /** Reads preserved legacy execution values for explicit recovery.
   * @param sessionId Stable session identity to inspect.
   * @param signal Optional request cancellation signal.
   * @returns Sanitized records requiring operator review.
   */
  getLegacyExecutionRecords(
    sessionId: SessionId,
    signal?: AbortSignal,
  ): Promise<readonly LegacyExecutionRecordView[]> {
    const request: LegacyExecutionRecordsRequest = { session: { id: sessionId } };
    return this.transport
      .request<unknown>("legacy_execution_records", request, signal)
      .then(normalizeLegacyExecutionRecords);
  }

  /** Maps one exact legacy session choice to an active upstream model.
   * @param sessionId Stable session identity.
   * @param expectedRevision Selection revision observed by the operator.
   * @param legacyPath Exact path returned by `getLegacyExecutionRecords`.
   * @param providerId Active upstream provider identity.
   * @param modelId Provider-scoped active model identity.
   * @param thinkingLevel Optional active thinking-level identity.
   * @param signal Optional request cancellation signal.
   * @returns Updated shared execution selection.
   */
  mapLegacyExecutionSelection(
    sessionId: SessionId,
    expectedRevision: number,
    legacyPath: string,
    providerId: string,
    modelId: string,
    thinkingLevel?: string,
    signal?: AbortSignal,
  ): Promise<SessionExecutionSelection> {
    const request: LegacyExecutionMappingRequest = {
      session: { id: sessionId },
      expectedRevision,
      legacyPath,
      providerId,
      modelId,
      ...(thinkingLevel === undefined ? {} : { thinkingLevel }),
    };
    return this.transport
      .request<unknown>(
        "legacy_execution_mapping",
        {
          session: request.session,
          expected_revision: request.expectedRevision,
          legacy_path: request.legacyPath,
          provider_id: request.providerId,
          model_id: request.modelId,
          ...(request.thinkingLevel === undefined ? {} : { thinking_level: request.thinkingLevel }),
        },
        signal,
      )
      .then(normalizeExecutionSelection);
  }

  /** Maps one current legacy plan assignment to an active upstream model.
   * @param sessionId Stable session identity.
   * @param legacyPath Exact path returned by `getLegacyExecutionRecords`.
   * @param expectedPlanRevision Plan revision observed by the operator.
   * @param providerId Active upstream provider identity.
   * @param modelId Provider-scoped active model identity.
   * @param thinkingLevel Optional active thinking-level identity.
   * @param signal Optional request cancellation signal.
   * @returns Updated durable plan projection.
   */
  mapLegacyPlanAssignment(
    sessionId: SessionId,
    legacyPath: string,
    expectedPlanRevision: number,
    providerId: string,
    modelId: string,
    thinkingLevel?: string,
    signal?: AbortSignal,
  ): Promise<SessionPlan> {
    const request: LegacyPlanAssignmentMappingRequest = {
      session: { id: sessionId },
      legacyPath,
      expectedPlanRevision,
      providerId,
      modelId,
      ...(thinkingLevel === undefined ? {} : { thinkingLevel }),
    };
    return this.transport
      .request<unknown>(
        "legacy_plan_assignment_mapping",
        {
          session: request.session,
          legacy_path: request.legacyPath,
          expected_plan_revision: request.expectedPlanRevision,
          provider_id: request.providerId,
          model_id: request.modelId,
          ...(request.thinkingLevel === undefined ? {} : { thinking_level: request.thinkingLevel }),
        },
        signal,
      )
      .then(normalizeSessionPlan);
  }
}
