/** @file Defines long-running operations and exhaustive public events. */
import type { EntityId, OperationId, SnapshotId } from "./ids";
/** Operation lifecycle states. */
export type OperationStatus = "queued" | "running" | "completed" | "failed" | "cancelled";
/** Public operation handle and state. */
export interface Operation {
  operationId: OperationId;
  kind: string;
  status: OperationStatus;
  progress?: number;
  resultSnapshotId?: SnapshotId;
  error?: string;
  output?: string;
}
/** Event envelope with exhaustive discriminant. */
export type ProtocolEvent =
  | { kind: "snapshotAdvanced"; snapshotId: SnapshotId }
  | { kind: "entityChanged"; entityId: EntityId; snapshotId: SnapshotId }
  | { kind: "projectionChanged"; projectionId: string }
  | { kind: "branchChanged"; branchId: string }
  | { kind: "migrationCompleted"; migrationId: string }
  | { kind: "validationCompleted"; operationId: OperationId }
  | { kind: "operationProgress"; operationId: OperationId; progress: number }
  | { kind: "agentOutput"; operationId: OperationId; text: string }
  | { kind: "operationFailed"; operationId: OperationId; error: string }
  | { kind: "operationCancelled"; operationId: OperationId };
