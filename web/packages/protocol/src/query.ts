/** @file Defines transport-neutral query request vocabulary. */
import type { EntityId, SnapshotId, TypeId } from "./ids";
import type { CollectionPage, ObjectDescription, ObjectSummary } from "./objects";
/** Resolves a query against an exact snapshot or named root. */
export type SnapshotSelector =
  { kind: "exact"; snapshotId: SnapshotId } | { kind: "root"; name: string } | { kind: "current" };
/** Requests semantic inspection. */
export interface InspectRequest {
  entityId: EntityId;
  snapshot: SnapshotSelector;
  detail: "compact" | "deep";
}
/** Lists or follows relation children. */
export interface ListRequest {
  entityId: EntityId;
  snapshot: SnapshotSelector;
  relation: string;
  cursor?: string;
}
/** Searches by semantic text or type. */
export interface FindRequest {
  snapshot: SnapshotSelector;
  text?: string;
  typeId?: TypeId;
  cursor?: string;
}
/** Declarative exact query request. */
export interface QueryRequest {
  snapshot: SnapshotSelector;
  expression: QueryExpression;
}
/** Minimal structured query expression. */
export type QueryExpression =
  | { kind: "type"; typeId: TypeId }
  | { kind: "label"; contains: string }
  | { kind: "and"; terms: QueryExpression[] };
/** Requests history for one entity. */
export interface HistoryRequest {
  entityId: EntityId;
  from?: SnapshotSelector;
  to?: SnapshotSelector;
}
/** Requests a semantic diff. */
export interface DiffRequest {
  base: SnapshotId;
  result: SnapshotId;
  entityId?: EntityId;
}
/** Union of supported read requests. */
export type ReadRequest =
  InspectRequest | ListRequest | FindRequest | QueryRequest | HistoryRequest | DiffRequest;
/** Union of read response shapes. */
export type ReadResponse = ObjectDescription | CollectionPage | ObjectSummary[];
