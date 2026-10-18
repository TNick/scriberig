/** @file Defines an HTTP transport without hard-coded server endpoint policy. */
import {
  ServerProtocolError,
  TransportError,
  type SemanticTransport,
  type TransportConnectionState,
} from "./transport";
import type {
  BranchId,
  OperationId,
  ProtocolEvent,
  ProtocolError,
  SnapshotId,
} from "@scriberig/protocol";
import { SseParser } from "./sse-parser";

type WireEvent = {
  sequence?: unknown;
  kind?: unknown;
  entity?: { id?: unknown };
  operation?: { id?: unknown };
  snapshot?: { id?: unknown };
  metadata?: unknown;
};

/** Unwraps a Rust response envelope while preserving direct test transports.
 * @param body Decoded HTTP response body.
 * @returns The response payload or the original body.
 * @throws When the server returns a structured protocol error.
 */
function unwrapResponse(body: unknown): unknown {
  if (typeof body !== "object" || body === null || !("response" in body)) return body;
  const response = (body as { response?: unknown }).response;
  if (typeof response !== "object" || response === null) return body;
  const entries = Object.entries(response as Record<string, unknown>);
  if (entries.length !== 1) return body;
  const entry = entries[0];
  if (entry === undefined) return body;
  const [kind, value] = entry;
  if (kind === "Error") throw new ServerProtocolError(value as ProtocolError);
  return value;
}

/**
 * Reads one string-valued field from a server event's metadata.
 * @param event Server event envelope.
 * @param key Metadata key to read.
 * @returns The string metadata value, when present.
 */
function metadataValue(event: WireEvent, key: string): string | undefined {
  if (!Array.isArray(event.metadata)) return undefined;
  const pair = event.metadata.find(
    (value): value is [unknown, unknown] =>
      Array.isArray(value) && value.length === 2 && value[0] === key,
  );
  if (!pair || typeof pair[1] !== "string") return undefined;
  return pair[1];
}

/**
 * Converts one server event envelope to the browser event contract.
 * @param value Unknown decoded server value.
 * @returns The normalized protocol event, when recognized.
 */
export function normalizeProtocolEvent(value: unknown): ProtocolEvent | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const event = value as WireEvent;
  if (typeof event.kind !== "string") return undefined;
  const sequence = typeof event.sequence === "number" ? event.sequence : undefined;
  const withSequence = (result: ProtocolEvent): ProtocolEvent =>
    sequence === undefined ? result : { ...result, sequence };
  if (event.kind === "snapshotAdvanced") return value as ProtocolEvent;
  const operationId = event.operation?.id;
  const snapshotId = event.snapshot?.id;
  const entityId = event.entity?.id;
  switch (event.kind) {
    case "SnapshotChanged":
      return typeof snapshotId === "string"
        ? withSequence({ kind: "snapshotAdvanced", snapshotId: snapshotId as SnapshotId })
        : undefined;
    case "EntityChanged":
      return typeof entityId === "string"
        ? withSequence({
            kind: "entityChanged",
            entityId: entityId as never,
            ...(typeof snapshotId === "string" ? { snapshotId: snapshotId as SnapshotId } : {}),
          })
        : undefined;
    case "ProjectionChanged":
      return typeof entityId === "string"
        ? withSequence({ kind: "projectionChanged", projectionId: entityId })
        : undefined;
    case "BranchChanged": {
      const branchId = metadataValue(event, "branch");
      return branchId === undefined ? undefined : withSequence({ kind: "branchChanged", branchId });
    }
    case "SessionChanged":
      return typeof entityId === "string"
        ? withSequence({ kind: "sessionChanged", sessionId: entityId as never })
        : undefined;
    case "SessionSelectionChanged": {
      const activityKind = metadataValue(event, "activity_kind");
      const summary = metadataValue(event, "summary");
      return typeof entityId === "string" && activityKind !== undefined && summary !== undefined
        ? withSequence({
            kind: "sessionSelectionChanged",
            sessionId: entityId as never,
            activityKind,
            summary,
          })
        : undefined;
    }
    case "MigrationCreated": {
      const migrationId = metadataValue(event, "migration");
      return migrationId === undefined
        ? undefined
        : withSequence({ kind: "migrationCreated", migrationId });
    }
    case "OperationStarted":
      return typeof operationId === "string"
        ? withSequence({ kind: "operationStarted", operationId: operationId as OperationId })
        : undefined;
    case "OperationProgressed": {
      if (typeof operationId !== "string") return undefined;
      const percent = Number(metadataValue(event, "percent") ?? 0);
      return withSequence({
        kind: "operationProgress",
        operationId: operationId as OperationId,
        progress: Number.isFinite(percent) ? percent / 100 : 0,
      });
    }
    case "OperationCompleted":
      return typeof operationId === "string"
        ? withSequence({ kind: "operationCompleted", operationId: operationId as OperationId })
        : undefined;
    case "OperationFailed": {
      const error = metadataValue(event, "error");
      return typeof operationId === "string" && error !== undefined
        ? withSequence({ kind: "operationFailed", operationId: operationId as OperationId, error })
        : undefined;
    }
    case "OperationCancelled":
      return typeof operationId === "string"
        ? withSequence({ kind: "operationCancelled", operationId: operationId as OperationId })
        : undefined;
    case "ValidationCompleted":
      return typeof operationId === "string"
        ? withSequence({ kind: "validationCompleted", operationId: operationId as OperationId })
        : undefined;
    case "AgentOutputProduced": {
      const text = metadataValue(event, "output");
      return typeof operationId === "string" && text !== undefined
        ? withSequence({ kind: "agentOutput", operationId: operationId as OperationId, text })
        : undefined;
    }
    case "AgentOutputDelta": {
      const text = metadataValue(event, "text");
      return typeof operationId === "string" && text !== undefined
        ? withSequence({ kind: "modelTextDelta", operationId: operationId as OperationId, text })
        : undefined;
    }
    case "AgentActivity": {
      const branchId = metadataValue(event, "branch");
      const activityKind = metadataValue(event, "activity_kind");
      const state = metadataValue(event, "state");
      const summary = metadataValue(event, "summary");
      return typeof operationId === "string" &&
        branchId !== undefined &&
        activityKind !== undefined &&
        state !== undefined &&
        summary !== undefined
        ? withSequence({
            kind: "agentActivity",
            operationId: operationId as OperationId,
            branchId: branchId as BranchId,
            activityKind,
            state,
            summary,
          })
        : undefined;
    }
    default:
      return undefined;
  }
}
/** Fetch-backed semantic transport with centralized operation mapping.
 *
 * Public members: `request`, `subscribe`.
 * Private members: base URL and authentication header provider.
 */
export class HttpTransport implements SemanticTransport {
  /** Creates an HTTP transport. */ constructor(
    private readonly baseUrl: string,
    private readonly headers: () => Record<string, string> = () => ({}),
  ) {}
  /** Sends one operation to the protocol dispatcher. */ async request<TResponse>(
    operation: string,
    payload: unknown,
    signal?: AbortSignal,
  ): Promise<TResponse> {
    let response: Response;
    try {
      response = await fetch(`${this.baseUrl}/protocol/${operation}`, {
        method: "POST",
        headers: { "content-type": "application/json", ...this.headers() },
        body: JSON.stringify(payload),
        ...(signal === undefined ? {} : { signal }),
      });
    } catch (error) {
      throw new TransportError("semantic transport request failed", { cause: error });
    }
    const body: unknown = await response.json().catch(() => undefined);
    if (!response.ok)
      throw new ServerProtocolError(
        (body ?? { code: "operation_failure", message: response.statusText }) as ProtocolError,
      );
    return unwrapResponse(body) as TResponse;
  }
  /** Consumes resumable protocol events through the SSE endpoint. */ subscribe(
    listener: (event: ProtocolEvent) => void,
    signal?: AbortSignal,
    onStateChange?: (state: TransportConnectionState) => void,
  ): () => void {
    return this.subscribeSse(listener, signal, onStateChange);
  }

  /** Opens the resumable Scriberig SSE endpoint until cancelled. */
  subscribeSse(
    listener: (event: ProtocolEvent) => void,
    signal?: AbortSignal,
    onStateChange?: (state: TransportConnectionState) => void,
  ): () => void {
    let active = true;
    let after: string | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const controller = new AbortController();
    const abort = () => {
      active = false;
      controller.abort();
      if (timer !== undefined) clearTimeout(timer);
    };
    if (signal?.aborted) abort();
    signal?.addEventListener("abort", abort, { once: true });

    const connect = async (): Promise<void> => {
      if (!active) return;
      try {
        const query = after === undefined ? "" : `?after=${encodeURIComponent(after)}`;
        const response = await fetch(`${this.baseUrl}/protocol/events${query}`, {
          headers: { accept: "text/event-stream", ...this.headers() },
          signal: controller.signal,
        });
        if (!response.ok || response.body === null) {
          throw new TransportError("event stream request failed");
        }
        onStateChange?.("connected");
        const reader = response.body.getReader();
        const decoder = new TextDecoder();
        const parser = new SseParser();
        while (active) {
          const chunk = await reader.read();
          if (chunk.done) break;
          for (const frame of parser.push(decoder.decode(chunk.value, { stream: true }))) {
            if (frame.id !== undefined) after = frame.id;
            if (frame.data.length === 0) continue;
            const event = normalizeProtocolEvent(JSON.parse(frame.data));
            if (event !== undefined) listener(event);
          }
        }
        for (const frame of parser.flush()) {
          if (frame.id !== undefined) after = frame.id;
          if (frame.data.length === 0) continue;
          const event = normalizeProtocolEvent(JSON.parse(frame.data));
          if (event !== undefined) listener(event);
        }
      } catch {
        if (!active) return;
        onStateChange?.("disconnected");
      }
      if (active) timer = setTimeout(() => void connect(), 1_000);
    };
    void connect();
    return () => {
      active = false;
      controller.abort();
      if (timer !== undefined) clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
    };
  }
}
