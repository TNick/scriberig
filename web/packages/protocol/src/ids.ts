/** @file Defines stable transport-safe semantic identifiers. */

/** Opaque entity identifier. */
export type EntityId = string & { readonly __entityId: unique symbol };
/** Opaque revision identifier. */
export type RevisionId = string & { readonly __revisionId: unique symbol };
/** Opaque snapshot identifier. */
export type SnapshotId = string & { readonly __snapshotId: unique symbol };
/** Opaque migration identifier. */
export type MigrationId = string & { readonly __migrationId: unique symbol };
/** Opaque operation identifier. */
export type OperationId = string & { readonly __operationId: unique symbol };
/** Opaque projection identifier. */
export type ProjectionId = string & { readonly __projectionId: unique symbol };
/** Opaque session identifier. */
export type SessionId = string & { readonly __sessionId: unique symbol };
/** Opaque branch identifier. */
export type BranchId = string & { readonly __branchId: unique symbol };
/** Opaque semantic type identifier. */
export type TypeId = string & { readonly __typeId: unique symbol };
/** Opaque relation type identifier. */
export type RelationTypeId = string & { readonly __relationTypeId: unique symbol };

/** Creates a random session identifier in the canonical Rust entity format.
 * @returns A 128-bit random `ent_` identifier branded as a session id.
 * @throws When the runtime does not provide a cryptographically secure random source.
 */
export function newSessionId(): SessionId {
  const cryptoApi = globalThis.crypto;
  if (cryptoApi === undefined || typeof cryptoApi.getRandomValues !== "function") {
    throw new Error("secure session identifiers require Web Crypto");
  }
  const bytes = cryptoApi.getRandomValues(new Uint8Array(16));
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `ent_${suffix}` as SessionId;
}
