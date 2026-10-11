/** @file Defines an HTTP transport without hard-coded server endpoint policy. */
import { ServerProtocolError, TransportError, type SemanticTransport } from "./transport";
import type { ProtocolEvent, ProtocolError } from "@scriberig/protocol";
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
    return body as TResponse;
  }
  /** Subscribes to events; streaming remains transport-owned. */ subscribe(
    _listener: (event: ProtocolEvent) => void,
    _signal?: AbortSignal,
  ): () => void {
    return () => undefined;
  }
}
