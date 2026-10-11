/** @file Defines progressive-disclosure semantic object summaries. */
import type { EntityId, RevisionId, SnapshotId, TypeId, RelationTypeId } from "./ids";
/** Identifies an entity at an exact snapshot. */
export interface EntityRef {
  entityId: EntityId;
  snapshotId: SnapshotId;
  revisionId?: RevisionId;
}
/** Describes an immutable revision reference. */
export interface RevisionRef {
  entityId: EntityId;
  revisionId: RevisionId;
}
/** Describes an immutable snapshot reference. */
export interface SnapshotRef {
  snapshotId: SnapshotId;
}
/** Compact semantic object summary. */
export interface ObjectSummary {
  ref: EntityRef;
  typeId: TypeId;
  label?: string;
  childCount?: number;
}
/** Public semantic description of an object. */
export interface ObjectDescription {
  summary: ObjectSummary;
  text?: string;
}
/** One relation edge summary. */
export interface RelationSummary {
  relationType: RelationTypeId;
  target: EntityRef;
  position?: number;
}
/** A page of ordered semantic children. */
export interface CollectionPage {
  items: ObjectSummary[];
  nextCursor?: string;
}
/** Trust and provenance metadata without private payloads. */
export interface ProvenanceSummary {
  source: string;
  producer?: string;
  inputRefs: RevisionRef[];
}
