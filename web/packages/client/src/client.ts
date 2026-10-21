/** @file Defines the public semantic client over an injected transport. */
import type {
  CapabilitiesResponse,
  DiffRequest,
  FindRequest,
  HistoryRequest,
  InspectRequest,
  ListRequest,
  MigrationDetail,
  Operation,
  AgentAcceptedResponse,
  AgentContinueRequest,
  AgentStartRequest,
  BranchActivityPage,
  BranchChildrenPage,
  BranchId,
  BranchResultReference,
  SessionId,
  SessionReadResponse,
  ModelRequestDetailResponse,
  ToolResultDetailResponse,
  AgentResult,
  AgentEvent,
  ProtocolEvent,
  QueryRequest,
  ReadResponse,
  ModelLibraryResponse,
  SessionExecutionSelection,
  AgentPresentationPreferences,
  AgentPresentationPreferencesResponse,
} from "@scriberig/protocol";
import { toAgentEvent } from "./agent-events";
import { subscribeEvents, type EventSubscription } from "./event-stream";
import { normalizeAcceptedAgent, normalizeOperation } from "./response-normalizers";
import {
  activityFromSession,
  branchFromSession,
  childrenFromSession,
  normalizeSession,
  resultFromSession,
} from "./session-views";
import { normalizeExecutionSelection, normalizeModelLibrary } from "./model-views";
import { readPresentationPreferences, replacePresentationPreferences } from "./presentation-views";
import type { SemanticTransport, TransportConnectionState } from "./transport";

/** Framework-independent client for shared semantic operations.
 *
 * Public members: semantic operation methods and `events`.
 * Private members: injected transport.
 */
export class ScriberigClient {
  /** Creates a client over an injected transport. */ constructor(
    private readonly transport: SemanticTransport,
  ) {}
  /** Inspects one entity. */ inspect(
    request: InspectRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("inspect", request, signal);
  }
  /** Lists or follows relation members. */ list(
    request: ListRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("list", request, signal);
  }
  /** Finds semantic objects. */ find(
    request: FindRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("find", request, signal);
  }
  /** Executes a structured query. */ query(
    request: QueryRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("query", request, signal);
  }
  /** Reads entity history. */ history(
    request: HistoryRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("history", request, signal);
  }
  /** Reads one canonical migration proposal detail. */
  migrationDetail(migrationId: string, signal?: AbortSignal): Promise<MigrationDetail> {
    return this.transport.request("migration", { migrationId }, signal);
  }
  /** Computes a semantic diff. */ diff(
    request: DiffRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("diff", request, signal);
  }
  /** Discovers runtime capabilities. */ capabilities(
    signal?: AbortSignal,
  ): Promise<CapabilitiesResponse> {
    return this.transport.request("capabilities", {}, signal);
  }
  /** Reads an operation lifecycle state. */ operation(
    operationId: string,
    signal?: AbortSignal,
  ): Promise<Operation> {
    return this.transport
      .request("operation", { operation: { id: operationId } }, signal)
      .then(normalizeOperation);
  }
  /** Reads an operation lifecycle state through the design-level method name. */
  getOperation(operationId: string, signal?: AbortSignal): Promise<Operation> {
    return this.operation(operationId, signal);
  }
  /** Executes a declared application command through the public boundary. */
  command<TPayload>(kind: string, payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.transport.request("command", { kind, payload }, signal).then(normalizeOperation);
  }
  /** Executes one typed project onboarding operation. */
  project<TResponse>(
    operation: string,
    payload: unknown,
    signal?: AbortSignal,
  ): Promise<TResponse> {
    return this.transport.request("project", { operation, payload }, signal);
  }
  /** Opens or updates a branch through the public application boundary. */
  branch<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("branch", payload, signal);
  }
  /** Proposes a migration through the public application boundary. */
  migrate<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("migrate", payload, signal);
  }
  /** Validates a migration proposal through the public application boundary. */
  validate<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("validate", payload, signal);
  }
  /** Renders a projection through the public application boundary. */
  render<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("render", payload, signal);
  }
  /** Opens or updates a session through the public application boundary. */
  session<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("session", payload, signal);
  }
  /** Starts an agent operation through the public application boundary. */
  agent<TPayload>(payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.command("agent", payload, signal);
  }
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
  /** Reads a previously captured safe model request without invoking a provider. */
  getModelRequestDetail(
    sessionId: SessionId,
    branchId: BranchId,
    invocationId: string,
    signal?: AbortSignal,
  ): Promise<ModelRequestDetailResponse> {
    return this.transport.request(
      "model_request_detail",
      {
        session: { id: sessionId },
        branch: { id: branchId },
        invocation: invocationId,
      },
      signal,
    );
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
   * @returns Safe model descriptors and curated profiles.
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
