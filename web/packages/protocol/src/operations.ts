/** @file Defines long-running operations and exhaustive public events. */
import type { BranchId, EntityId, OperationId, SessionId, SnapshotId } from "./ids";
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
  sessionId?: SessionId;
  branchId?: BranchId;
  result?: import("./sessions").AgentResult;
  failure?: import("./sessions").AgentFailure;
  /** Whether the shared lifecycle currently permits cancellation. */
  cancellable?: boolean;
}
/** Event envelope with exhaustive discriminant. */
export type ProtocolEvent = { sequence?: number } & (
  | { kind: "snapshotAdvanced"; snapshotId: SnapshotId }
  | { kind: "entityChanged"; entityId: EntityId; snapshotId?: SnapshotId }
  | { kind: "projectionChanged"; projectionId: string }
  | { kind: "branchChanged"; branchId: string }
  | { kind: "sessionChanged"; sessionId: string }
  | { kind: "migrationCompleted"; migrationId: string }
  | { kind: "migrationCreated"; migrationId: string }
  | { kind: "validationCompleted"; operationId: OperationId }
  | { kind: "operationStarted"; operationId: OperationId }
  | { kind: "operationCompleted"; operationId: OperationId }
  | { kind: "operationProgress"; operationId: OperationId; progress: number }
  | { kind: "agentOutput"; operationId: OperationId; text: string }
  | { kind: "operationFailed"; operationId: OperationId; error: string }
  | { kind: "operationCancelled"; operationId: OperationId }
);

/** Provider-neutral agent lifecycle event with a monotonic sequence. */
export type AgentEvent =
  | { kind: "branchCreated"; sequence: number; branch: import("./sessions").BranchDescriptor }
  | { kind: "branchChanged"; sequence: number; branchId: BranchId }
  | { kind: "modelTextDelta"; sequence: number; operationId: OperationId; text: string }
  | { kind: "agentOutputCommitted"; sequence: number; operationId: OperationId }
  | { kind: "operationProgress"; sequence: number; operationId: OperationId; phase?: string }
  | { kind: "operationCompleted"; sequence: number; operationId: OperationId }
  | {
      kind: "operationFailed";
      sequence: number;
      operationId: OperationId;
      failure: import("./sessions").AgentFailure;
    }
  | { kind: "operationCancelled"; sequence: number; operationId: OperationId };
