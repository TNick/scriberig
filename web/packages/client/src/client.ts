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
  AgentResult,
  AgentEvent,
  ProtocolEvent,
  QueryRequest,
  ReadResponse,
} from "@scriberig/protocol";
import { toAgentEvent } from "./agent-events";
import { subscribeEvents, type EventSubscription } from "./event-stream";
import { normalizeAcceptedAgent, normalizeOperation } from "./response-normalizers";
import { branchActivity, branchDescriptor, normalizeSession } from "./session-views";
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
  /** Reads one branch from the authoritative session tree. */
  getBranch(
    sessionId: SessionId,
    branchId: BranchId,
    signal?: AbortSignal,
  ): Promise<BranchChildrenPage["items"][number]> {
    return this.getSession(sessionId, signal).then((session) => {
      const branch = session.branches.find((candidate) => candidate.id === branchId);
      if (branch === undefined) throw new Error("branch is not present in session");
      return branchDescriptor(branch);
    });
  }
  /** Reads child branch descriptors without loading their full activity. */
  listBranchChildren(
    sessionId: SessionId,
    branchId: BranchId,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<BranchChildrenPage> {
    void cursor;
    return this.getSession(sessionId, signal).then((session) => {
      const branch = session.branches.find((candidate) => candidate.id === branchId);
      if (branch === undefined) throw new Error("branch is not present in session");
      const children = new Set(branch.children);
      return {
        branchId,
        items: session.branches
          .filter((candidate) => children.has(candidate.id))
          .map(branchDescriptor),
      };
    });
  }
  /** Reads one bounded activity page for a branch. */
  listBranchActivity(
    sessionId: SessionId,
    branchId: BranchId,
    cursor?: string,
    signal?: AbortSignal,
  ): Promise<BranchActivityPage> {
    void cursor;
    return this.getSession(sessionId, signal).then((session) => {
      const branch = session.branches.find((candidate) => candidate.id === branchId);
      if (branch === undefined) throw new Error("branch is not present in session");
      return branchActivity(branch);
    });
  }
  /** Reads a durable branch result by reference. */
  getBranchResult(
    sessionId: SessionId,
    resultId: string,
    signal?: AbortSignal,
  ): Promise<BranchResultReference> {
    return this.getSession(sessionId, signal).then((session) => {
      const branch = session.branches.find((candidate) => candidate.result?.id === resultId);
      if (branch?.result === undefined) throw new Error("branch result is not present in session");
      return {
        branchId: branch.id,
        resultId: branch.result.id,
        handles: branch.result.references,
      };
    });
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
