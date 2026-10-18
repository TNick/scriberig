/** @file Projects protocol events into provider-neutral agent events. */
import type { AgentEvent, ProtocolEvent } from "@scriberig/protocol";

/** Converts one public operation event into an agent event.
 * @param event Normalized protocol event.
 * @param sequence Fallback sequence for legacy transports.
 * @returns Agent event and resulting sequence.
 */
export function toAgentEvent(
  event: ProtocolEvent,
  sequence: number,
): { event?: AgentEvent; sequence: number } {
  const nextSequence = Math.max(sequence + 1, event.sequence ?? sequence + 1);
  switch (event.kind) {
    case "agentOutput":
      return {
        sequence: nextSequence,
        event: {
          kind: "agentOutputCommitted",
          sequence: nextSequence,
          operationId: event.operationId,
        },
      };
    case "branchChanged":
      return {
        sequence: nextSequence,
        event: { kind: "branchChanged", sequence: nextSequence, branchId: event.branchId as never },
      };
    case "agentActivity":
      return {
        sequence: nextSequence,
        event: {
          kind: "transientActivity",
          sequence: nextSequence,
          operationId: event.operationId,
          branchId: event.branchId,
          activityKind: event.activityKind,
          state: event.state,
          summary: event.summary,
        },
      };
    case "sessionChanged":
      return {
        sequence: nextSequence,
        event: {
          kind: "branchChanged",
          sequence: nextSequence,
          branchId: event.sessionId as never,
        },
      };
    case "sessionSelectionChanged":
      return {
        sequence: nextSequence,
        event: {
          kind: "sessionSelectionChanged",
          sequence: nextSequence,
          sessionId: event.sessionId as never,
          activityKind: event.activityKind,
          summary: event.summary,
        },
      };
    case "operationProgress":
      return {
        sequence: nextSequence,
        event: {
          kind: "operationProgress",
          sequence: nextSequence,
          operationId: event.operationId,
        },
      };
    case "operationCompleted":
      return {
        sequence: nextSequence,
        event: {
          kind: "operationCompleted",
          sequence: nextSequence,
          operationId: event.operationId,
        },
      };
    case "operationFailed":
      return {
        sequence: nextSequence,
        event: {
          kind: "operationFailed",
          sequence: nextSequence,
          operationId: event.operationId,
          failure: { category: "operation_failure", message: event.error },
        },
      };
    case "operationCancelled":
      return {
        sequence: nextSequence,
        event: {
          kind: "operationCancelled",
          sequence: nextSequence,
          operationId: event.operationId,
        },
      };
    default:
      return { sequence: nextSequence };
  }
}
