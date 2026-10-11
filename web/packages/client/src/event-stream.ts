/** @file Defines event subscription lifecycle independent of UI frameworks. */
import type { ProtocolEvent } from "@scriberig/protocol";
import type { SemanticTransport } from "./transport";
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
): EventSubscription {
  let state: EventStreamState = "connected";
  const unsubscribe = transport.subscribe(listener, signal);
  const close = () => {
    if (state === "connected") {
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
