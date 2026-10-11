/** @file Defines stable transport-safe semantic identifiers. */

/** Opaque entity identifier. */
export type EntityId = string & { readonly __entityId: unique symbol };
/** Opaque revision identifier. */
export type RevisionId = string & { readonly __revisionId: unique symbol };
/** Opaque snapshot identifier. */
export type SnapshotId = string & { readonly __snapshotId: unique symbol };
