/** @file Defines the framework-independent semantic transport contract. */
import type { ProtocolError, ProtocolEvent } from "@scriberig/protocol";
/** Typed transport failure separate from semantic protocol errors.
 *
 * Public members: `name`, `message`.
 * Private members: none.
 */
export class TransportError extends Error {
  /** Creates a transport error. */ constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "TransportError";
  }
}
/** A transport operation with typed payload and result. */
export interface SemanticTransport {
  request<TResponse>(operation: string, payload: unknown, signal?: AbortSignal): Promise<TResponse>;
  subscribe(listener: (event: ProtocolEvent) => void, signal?: AbortSignal): () => void;
}
/** Semantic error returned by a server.
 *
 * Public members: `error`.
 * Private members: none.
 */
export class ServerProtocolError extends Error {
  /** Structured protocol error. */ readonly error: ProtocolError;
  /** Creates a server protocol error. */ constructor(error: ProtocolError) {
    super(error.message);
    this.name = "ServerProtocolError";
    this.error = error;
  }
}
