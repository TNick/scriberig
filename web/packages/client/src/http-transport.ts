/** @file Defines an HTTP transport without hard-coded server endpoint policy. */
import {
  ServerProtocolError,
  TransportError,
  type SemanticTransport,
  type TransportConnectionState,
} from "./transport";
import type { ProtocolError, ProtocolEvent } from "@scriberig/protocol";
import { SseParser } from "./sse-parser";
import { encodeRequest } from "./protocol-wire-requests";
import { normalizeProtocolEvent } from "./protocol-events";

export { normalizeProtocolEvent } from "./protocol-events";

const LEGACY_HTTP_OPERATIONS = new Set([
  "capabilities",
  "command",
  "inspect",
  "list",
  "migration",
  "project",
  "session",
  "subscribe",
]);

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
        headers: {
          "content-type": "application/json",
          accept: LEGACY_HTTP_OPERATIONS.has(operation)
            ? "application/json"
            : "application/vnd.scriberig.protocol+json",
          "x-scriberig-protocol": "1",
          ...this.headers(),
        },
        body: JSON.stringify(encodeRequest(operation, payload)),
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
