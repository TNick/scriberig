/** @file Implements generic semantic operations over an injected transport. */
import type {
  CapabilitiesResponse,
  DiffRequest,
  FindRequest,
  HistoryRequest,
  InspectRequest,
  ListRequest,
  MigrationDetail,
  Operation,
  ProtocolEvent,
  QueryRequest,
  ReadResponse,
} from "@scriberig/protocol";
import { subscribeEvents, type EventSubscription } from "./event-stream";
import {
  normalizeCollectionPage,
  normalizeObjectDescription,
  normalizeOperation,
} from "./response-normalizers";
import type { SemanticTransport, TransportConnectionState } from "./transport";

/** Framework-independent semantic operations over an injected transport.
 *
 * Public members: semantic operation methods and event subscription.
 * Private members: none.
 */
export class SemanticClient {
  /** Creates a client over an injected transport. */
  constructor(protected readonly transport: SemanticTransport) {}
  /** Inspects one entity. */ inspect(
    request: InspectRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("inspect", request, signal).then(normalizeObjectDescription);
  }
  /** Lists or follows relation members. */ list(
    request: ListRequest,
    signal?: AbortSignal,
  ): Promise<ReadResponse> {
    return this.transport.request("list", request, signal).then(normalizeCollectionPage);
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
    return this.transport
      .request<{ result: TResponse }>("project", { operation, payload }, signal)
      .then((response) => response.result);
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
}
