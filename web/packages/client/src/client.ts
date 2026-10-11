/** @file Defines the public semantic client over an injected transport. */
import type {
  CapabilitiesResponse,
  DiffRequest,
  FindRequest,
  HistoryRequest,
  InspectRequest,
  ListRequest,
  Operation,
  ProtocolEvent,
  QueryRequest,
  ReadResponse,
  SnapshotId,
} from "@scriberig/protocol";
import { subscribeEvents, type EventSubscription } from "./event-stream";
import type { SemanticTransport } from "./transport";

/** Normalizes legacy and current operation response shapes.
 *
 * @param value Untrusted operation response from a transport.
 * @returns A normalized public operation.
 * @throws When the response is not an operation object.
 */
function normalizeOperation(value: unknown): Operation {
  if (typeof value !== "object" || value === null) {
    throw new Error("server returned an invalid operation");
  }
  const record = value as Record<string, unknown>;
  if (typeof record.operationId === "string") return record as unknown as Operation;
  const reference = record.reference as Record<string, unknown> | undefined;
  const progress = record.progress as Record<string, unknown> | undefined;
  const result = record.result_snapshot as Record<string, unknown> | undefined;
  const kind = record.kind;
  const status = record.status;
  const normalized: Operation = {
    operationId: String(reference?.id ?? "") as Operation["operationId"],
    kind: typeof kind === "string" ? kind.toLowerCase() : "unknown",
    status: (typeof status === "string" ? status.toLowerCase() : "failed") as Operation["status"],
  };
  if (typeof progress?.percent === "number") normalized.progress = progress.percent / 100;
  if (typeof result?.id === "string") {
    normalized.resultSnapshotId = result.id as SnapshotId;
  }
  if (typeof record.error === "string") normalized.error = record.error;
  return normalized;
}
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
    return this.transport.request("operation", { operationId }, signal);
  }
  /** Executes a declared application command through the public boundary. */
  command<TPayload>(kind: string, payload: TPayload, signal?: AbortSignal): Promise<Operation> {
    return this.transport.request("command", { kind, payload }, signal).then(normalizeOperation);
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
  /** Subscribes to semantic events. */ events(
    listener: (event: ProtocolEvent) => void,
    signal?: AbortSignal,
  ): EventSubscription {
    return subscribeEvents(this.transport, listener, signal);
  }
}
