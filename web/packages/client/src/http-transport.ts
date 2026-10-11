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
  /** Polls buffered events until the caller closes the subscription. */ subscribe(
    listener: (event: ProtocolEvent) => void,
    signal?: AbortSignal,
  ): () => void {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const abort = () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
    };
    signal?.addEventListener("abort", abort, { once: true });
    const poll = async (): Promise<void> => {
      if (!active) return;
      try {
        const events = await this.request<ProtocolEvent[]>("subscribe", { eventKinds: [] }, signal);
        if (!active) return;
        for (const event of events) listener(event);
      } catch {
        if (!active) return;
      }
      if (active) timer = setTimeout(() => void poll(), 1_000);
    };
    void poll();
    return () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
      signal?.removeEventListener("abort", abort);
    };
  }
}
