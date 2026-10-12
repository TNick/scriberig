/** @file Defines event subscription lifecycle independent of UI frameworks. */
import type { ProtocolEvent } from "@scriberig/protocol";
import type { SemanticTransport, TransportConnectionState } from "./transport";
/** Event stream lifecycle state. */
export type EventStreamState = "connected" | "disconnected";
/** A closable typed event subscription. */
export interface EventSubscription {
  readonly state: EventStreamState;
  close(): void;
}
/** Subscribes to semantic events from a transport. */
/**
 * @param transport Transport supplying events.
 * @param listener Callback for each event.
 * @param signal Optional cancellation signal.
 * @returns A closable event subscription.
 */
export function subscribeEvents(
  transport: SemanticTransport,
  listener: (event: ProtocolEvent) => void,
  signal?: AbortSignal,
  onStateChange?: (state: EventStreamState) => void,
): EventSubscription {
  let state: EventStreamState = "connected";
  let closed = false;
  const notifyStateChange = (next: TransportConnectionState): void => {
    state = next;
    onStateChange?.(next);
  };
  const unsubscribe = transport.subscribe(listener, signal, notifyStateChange);
  const close = () => {
    if (!closed) {
      closed = true;
      state = "disconnected";
      unsubscribe();
    }
  };
  return {
    get state() {
      return state;
    },
    close,
  };
}
