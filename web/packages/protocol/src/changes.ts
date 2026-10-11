/** @file Defines candidate, migration, diff, and validation protocol shapes. */
import type { EntityId, MigrationId, RevisionId, SnapshotId } from "./ids";
/** Compact migration metadata. */
export interface MigrationSummary {
  migrationId: MigrationId;
  base: SnapshotId;
  result: SnapshotId;
  source: string;
}
/** Detailed migration with affected entity revisions. */
export interface MigrationDetail extends MigrationSummary {
  changes: SemanticDiff[];
}
/** Proposed migration before canonical acceptance. */
export interface MigrationProposal {
  proposalId: string;
  base: SnapshotId;
  changes: SemanticDiff[];
}
/** One semantic change between exact revisions. */
export interface SemanticDiff {
  entityId: EntityId;
  before?: RevisionId;
  after?: RevisionId;
  kind: "created" | "removed" | "modified";
}
/** Structured validation outcome. */
export interface ValidationResult {
  status: "current" | "stale" | "failed" | "unknown";
  evidenceRefs: string[];
}
/** Structured semantic conflict. */
export interface Conflict {
  conflictId: string;
  entityId: EntityId;
  reason: string;
  resolvable: boolean;
}
/** Explicit stale-base condition. */
export interface StaleBase {
  expected: SnapshotId;
  actual: SnapshotId;
}
