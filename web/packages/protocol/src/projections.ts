/** @file Defines public projection metadata and editability. */
import type { EntityId, ProjectionId, SnapshotId } from "./ids";
/** Projection metadata exposed to generic clients. */
export interface ProjectionInfo {
  projectionId: ProjectionId;
  source: EntityId;
  styleId?: string;
  snapshotId: SnapshotId;
  editable: boolean;
  readOnlyReason?: string;
}
/** Maps a semantic entity to a projection location. */
export interface SemanticLocation {
  entityId: EntityId;
  path: string;
  start?: number;
  end?: number;
}
