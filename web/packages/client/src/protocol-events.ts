/** @file Normalizes protocol event envelopes for browser consumers. */
import type {
  BranchId,
  EntityId,
  OperationId,
  ProtocolEvent,
  SessionActivityItem,
  SnapshotId,
} from "@scriberig/protocol";

type WireEvent = {
  sequence?: unknown;
  kind?: unknown;
  entity?: { id?: unknown };
  operation?: { id?: unknown };
  snapshot?: { id?: unknown };
  activity_kind?: unknown;
  activity_state?: unknown;
  metadata?: unknown;
};

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
      const activityKind = selectionActivityKind(
        typeof event.activity_kind === "string" ? event.activity_kind : undefined,
      );
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
    case "PlanActivity": {
      const activityKind = sharedActivityKind(
        typeof event.activity_kind === "string" ? event.activity_kind : undefined,
      );
      const state = activityState(
        typeof event.activity_state === "string" ? event.activity_state : undefined,
      );
      const sessionId = metadataValue(event, "session");
      const nodeId = metadataValue(event, "node");
      const summary = metadataValue(event, "summary");
      return typeof entityId === "string" &&
        sessionId !== undefined &&
        activityKind !== undefined &&
        state !== undefined &&
        summary !== undefined
        ? withSequence({
            kind: "planActivity",
            sessionId: sessionId as never,
            planId: entityId as EntityId,
            ...(nodeId === undefined ? {} : { nodeId }),
            activityKind,
            state,
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
      const activityKind = sharedActivityKind(
        typeof event.activity_kind === "string" ? event.activity_kind : undefined,
      );
      const state = activityState(
        typeof event.activity_state === "string" ? event.activity_state : undefined,
      );
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

/** Validates a serialized shared activity kind before exposing it to clients.
 * @param value Wire value from event metadata.
 * @returns Typed activity kind, or undefined when the server sent an unknown value.
 * @throws Never; unrecognized values return undefined.
 */
function sharedActivityKind(value: string | undefined): SessionActivityItem["kind"] | undefined {
  const kinds: readonly SessionActivityItem["kind"][] = [
    "HumanInput",
    "AssistantOutput",
    "ReasoningSummary",
    "RuntimeStatus",
    "ModelInvocation",
    "ModelRequest",
    "ToolInvocation",
    "ToolResult",
    "BranchStarted",
    "BranchCompleted",
    "OperationProgress",
    "Validation",
    "ModelSelectionChanged",
    "ThinkingLevelChanged",
    "PlanCreated",
    "PlanRevised",
    "PlanNodeReady",
    "PlanNodeStarted",
    "PlanNodeCompleted",
    "PlanNodeFailed",
    "AssignmentChanged",
    "CoordinatorChanged",
    "DependencyStateChanged",
    "SemanticOperation",
    "ExternalProcess",
    "Error",
  ];
  return kinds.find((kind) => kind === value);
}

/** Validates serialized selection-change categories.
 * @param value Wire value from event metadata.
 * @returns A typed selection category, or undefined when unrecognized.
 * @throws Never; unrecognized values return undefined.
 */
function selectionActivityKind(
  value: string | undefined,
): "ModelSelectionChanged" | "ThinkingLevelChanged" | undefined {
  return value === "ModelSelectionChanged" || value === "ThinkingLevelChanged" ? value : undefined;
}

/** Validates serialized activity lifecycle states.
 * @param value Wire value from event metadata.
 * @returns A typed lifecycle state, or undefined when unrecognized.
 * @throws Never; unrecognized values return undefined.
 */
function activityState(value: string | undefined): SessionActivityItem["state"] | undefined {
  return value === "Running" || value === "Completed" || value === "Failed" || value === "Cancelled"
    ? value
    : undefined;
}
