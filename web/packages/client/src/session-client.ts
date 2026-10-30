/** @file Implements agent and durable session operations for the public client. */
import type {
  AgentAcceptedResponse,
  AgentContinueRequest,
  AgentStartRequest,
  BranchActivityPage,
  BranchChildrenPage,
  BranchId,
  BranchResultReference,
  SessionId,
  SessionReadResponse,
  SessionPlan,
  SessionPlanAttemptsRequest,
  SessionPlanAttemptsResponse,
  SessionPlanMutationRequest,
  ModelRequestDetailResponse,
  ModelRequestDetailPageRequest,
  ToolResultDetailResponse,
  AgentResult,
  AgentEvent,
  ModelLibraryResponse,
  SessionExecutionSelection,
  AgentPresentationPreferences,
  AgentPresentationPreferencesResponse,
  Operation,
  ProtocolEvent,
} from "@scriberig/protocol";
import { toAgentEvent } from "./agent-events";
import { subscribeEvents, type EventSubscription } from "./event-stream";
import {
  normalizeAcceptedAgent,
  normalizeModelRequestDetail,
  normalizeOperation,
} from "./response-normalizers";
import {
  activityFromSession,
  branchFromSession,
  childrenFromSession,
  normalizeSession,
  resultFromSession,
} from "./session-views";
import { normalizeExecutionSelection, normalizeModelLibrary } from "./model-views";
import { readPresentationPreferences, replacePresentationPreferences } from "./presentation-views";
import type { TransportConnectionState } from "./transport";
import { normalizeSessionPlan } from "./session-plan-views";
import { planMutationToWire } from "./session-plan-mutation-wire";
import { normalizePlanAttempts } from "./session-plan-attempts";
import { LegacyExecutionRecoveryClient } from "./legacy-execution-recovery-client";

/** Durable agent and session operations on top of semantic operations.
 *
 * Public members: agent, session, model, branch, and event methods.
 * Private members: inherited semantic transport.
 */
export class SessionClient extends LegacyExecutionRecoveryClient {
  /** Starts an agent operation and returns durable accepted handles. */
  startAgent(request: AgentStartRequest, signal?: AbortSignal): Promise<AgentAcceptedResponse> {
    return this.transport
      .request(
        "agent_start",
        {
          session: { id: request.sessionId },
          prompt: request.prompt,
          ...(request.selection === undefined
            ? {}
            : {
                selection: {
                  ...(request.selection.projectId === undefined
                    ? {}
                    : { project_id: request.selection.projectId }),
                  ...(request.selection.rootId === undefined
                    ? {}
                    : { root: { id: request.selection.rootId } }),
                  entity: { id: request.selection.entityId },
                  snapshot: { id: request.selection.snapshotId ?? "" },
                },
              }),
        },
        signal,
      )
      .then(normalizeAcceptedAgent);
  }
  /** Continues an agent operation on an existing durable branch. */
  continueAgent(
    request: AgentContinueRequest,
    signal?: AbortSignal,
  ): Promise<AgentAcceptedResponse> {
    return this.transport
      .request(
        "agent_continue",
        {
          session: { id: request.sessionId },
          branch: { id: request.branchId },
          prompt: request.prompt,
          ...(request.selection === undefined
            ? {}
            : {
                selection: {
                  ...(request.selection.projectId === undefined
                    ? {}
                    : { project_id: request.selection.projectId }),
                  ...(request.selection.rootId === undefined
                    ? {}
                    : { root: { id: request.selection.rootId } }),
                  entity: { id: request.selection.entityId },
                  snapshot: { id: request.selection.snapshotId ?? "" },
                },
              }),
        },
        signal,
      )
      .then(normalizeAcceptedAgent);
  }
  /** Cancels one shared operation through the public lifecycle boundary. */
  cancelOperation(operationId: string, signal?: AbortSignal): Promise<Operation> {
    return this.transport
      .request("cancel", { operation: { id: operationId } }, signal)
      .then(normalizeOperation);
  }
  /** Reads the authoritative operation result after a terminal event. */
  getAgentResult(operationId: string, signal?: AbortSignal): Promise<AgentResult> {
    return this.operation(operationId, signal).then((operation) => {
      if (operation.result === undefined) throw new Error("agent result is not available");
      return operation.result;
    });
  }
  /** Reads one session descriptor through the public client boundary. */
  getSession(sessionId: SessionId, signal?: AbortSignal): Promise<SessionReadResponse> {
    return this.transport
      .request("session", { session: { id: sessionId } }, signal)
      .then(normalizeSession);
  }
  /** Applies one typed semantic plan mutation through the shared app boundary.
   * @param request Plan identity, expected revision, and mutation.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative plan projection.
   */
  mutatePlan(request: SessionPlanMutationRequest, signal?: AbortSignal): Promise<SessionPlan> {
    return this.transport
      .request<unknown>(
        "plan_mutation",
        {
          session: request.session,
          plan: request.plan,
          expected_revision: request.expectedRevision,
          mutation: planMutationToWire(request.mutation),
        },
        signal,
      )
      .then(normalizeSessionPlan);
  }
  /** Reads one bounded, transcript-free page of plan-node attempts.
   * @param request Session, plan, node, and optional page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Safe attempt metadata and the next page cursor.
   */
  getPlanAttempts(
    request: SessionPlanAttemptsRequest,
    signal?: AbortSignal,
  ): Promise<SessionPlanAttemptsResponse> {
    return this.transport
      .request<unknown>(
        "plan_attempts",
        {
          session: request.session,
          plan: request.plan,
          node: request.node,
          ...(request.offset === undefined ? {} : { offset: request.offset }),
          ...(request.limit === undefined ? {} : { limit: request.limit }),
        },
        signal,
      )
      .then(normalizePlanAttempts);
  }
  /** Reads a previously captured safe model request without invoking a provider. */
  getModelRequestDetail(
    sessionId: SessionId,
    branchId: BranchId,
    invocationId: string,
    signal?: AbortSignal,
  ): Promise<ModelRequestDetailResponse> {
    return this.transport
      .request<unknown>(
        "model_request_detail",
        {
          session: { id: sessionId },
          branch: { id: branchId },
          invocation: invocationId,
        },
        signal,
      )
      .then(normalizeModelRequestDetail);
  }
  /** Reads one bounded page of the complete safe captured request text.
   * @param sessionId Session containing the invocation.
   * @param branchId Branch containing the invocation.
   * @param invocationId Stable invocation identity.
   * @param page Numeric Unicode-scalar page cursor and character budget.
   * @param signal Optional request cancellation signal.
   * @returns Safe request metadata and one bounded text page.
   */
  getModelRequestDetailPage(
    sessionId: SessionId,
    branchId: BranchId,
    invocationId: string,
    page: ModelRequestDetailPageRequest = {},
    signal?: AbortSignal,
  ): Promise<ModelRequestDetailResponse> {
    return this.transport
      .request<unknown>(
        "model_request_detail",
        {
          session: { id: sessionId },
          branch: { id: branchId },
          invocation: invocationId,
          offset: page.offset ?? 0,
          max_chars: page.maxChars ?? 16_384,
        },
        signal,
      )
      .then(normalizeModelRequestDetail);
  }
  /** Reads bounded safe detail for one durable tool result. */
  getToolResultDetail(
    sessionId: SessionId,
    branchId: BranchId,
    resultId: string,
    signal?: AbortSignal,
  ): Promise<ToolResultDetailResponse> {
    return this.transport.request(
      "tool_result_detail",
      {
        session: { id: sessionId },
        branch: { id: branchId },
        result: resultId,
      },
      signal,
    );
  }
  /** Reads the safe model library used by agent model and thinking controls.
   * @param signal Optional request cancellation signal.
   * @returns Safe upstream-derived model descriptors.
   */
  getModelLibrary(signal?: AbortSignal): Promise<ModelLibraryResponse> {
    return this.transport.request("model_library", {}, signal).then(normalizeModelLibrary);
  }
  /** Reads shared presentation defaults for the authenticated principal.
   * @param signal Optional request cancellation signal.
   * @returns Revisioned shared defaults.
   */
  getAgentPresentationPreferences(
    signal?: AbortSignal,
  ): Promise<AgentPresentationPreferencesResponse> {
    return readPresentationPreferences(this.transport, signal);
  }
  /** Replaces shared presentation defaults at the observed revision.
   * @param expectedRevision Last observed preference revision.
   * @param preferences Complete independent display choices.
   * @param signal Optional request cancellation signal.
   * @returns Updated revisioned defaults.
   */
  setAgentPresentationPreferences(
    expectedRevision: number,
    preferences: AgentPresentationPreferences,
    signal?: AbortSignal,
  ): Promise<AgentPresentationPreferencesResponse> {
    return replacePresentationPreferences(this.transport, expectedRevision, preferences, signal);
  }
  /** Selects an upstream provider/model pair for future session invocations.
   * @param sessionId Target durable session.
   * @param expectedRevision Selection mutation revision observed by the caller.
   * @param providerId Public upstream provider id to select.
   * @param modelId Public upstream model id to select.
   * @param signal Optional request cancellation signal.
   * @returns Updated shared execution selection.
   */
  setSessionModel(
    sessionId: SessionId,
    expectedRevision: number,
    providerId: string,
    modelId: string,
    signal?: AbortSignal,
  ): Promise<SessionExecutionSelection> {
    return this.transport
      .request(
        "session_model_selection",
        {
          session: { id: sessionId },
          expected_revision: expectedRevision,
          provider_id: providerId,
          model_id: modelId,
        },
        signal,
      )
      .then(normalizeExecutionSelection);
  }
  /** Selects an advertised thinking level for future session invocations.
   * @param sessionId Target durable session.
   * @param expectedRevision Selection mutation revision observed by the caller.
   * @param thinkingLevel Advertised capability id selected for the session.
   * @param signal Optional request cancellation signal.
   * @returns Updated shared execution selection.
   */
  setSessionThinkingLevel(
    sessionId: SessionId,
    expectedRevision: number,
    thinkingLevel: string,
    signal?: AbortSignal,
  ): Promise<SessionExecutionSelection> {
    return this.transport
      .request(
        "session_thinking_selection",
        {
          session: { id: sessionId },
          expected_revision: expectedRevision,
          thinking_level: thinkingLevel,
        },
        signal,
      )
      .then(normalizeExecutionSelection);
  }
  /** Reads one branch from the authoritative session tree. */
  getBranch(
    sessionId: SessionId,
    branchId: BranchId,
    signal?: AbortSignal,
  ): Promise<BranchChildrenPage["items"][number]> {
    return this.getSession(sessionId, signal).then((session) =>
      branchFromSession(session, branchId),
    );
  }
  /** Reads child branch descriptors without loading their full activity. */
  listBranchChildren(
    sessionId: SessionId,
    branchId: BranchId,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<BranchChildrenPage> {
    void cursor;
    return this.getSession(sessionId, signal).then((session) =>
      childrenFromSession(session, branchId),
    );
  }
  /** Reads one bounded activity page for a branch. */
  listBranchActivity(
    sessionId: SessionId,
    branchId: BranchId,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<BranchActivityPage> {
    void cursor;
    return this.getSession(sessionId, signal).then((session) =>
      activityFromSession(session, branchId),
    );
  }
  /** Reads a durable branch result by reference. */
  getBranchResult(
    sessionId: SessionId,
    resultId: string,
    signal?: AbortSignal,
  ): Promise<BranchResultReference> {
    return this.getSession(sessionId, signal).then((session) =>
      resultFromSession(session, resultId),
    );
  }
  /** Subscribes to semantic events. */ events(
    listener: (event: ProtocolEvent) => void,
    signal?: AbortSignal,
    onStateChange?: (state: TransportConnectionState) => void,
  ): EventSubscription {
    return subscribeEvents(this.transport, listener, signal, onStateChange);
  }
  /** Subscribes to the agent subset of the resumable event stream.
   * @param listener Agent event callback.
   * @param signal Optional cancellation signal.
   * @param onStateChange Optional connection-state callback.
   * @returns A closable event subscription.
   */
  subscribeAgentEvents(
    listener: (event: AgentEvent) => void,
    signal?: AbortSignal,
    onStateChange?: (state: TransportConnectionState) => void,
  ): EventSubscription {
    let sequence = 0;
    return this.events(
      (event) => {
        const result = toAgentEvent(event, sequence);
        sequence = result.sequence;
        if (result.event !== undefined) listener(result.event);
      },
      signal,
      onStateChange,
    );
  }
}
