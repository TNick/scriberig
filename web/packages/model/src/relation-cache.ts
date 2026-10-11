/** @file Caches relation pages scoped to immutable snapshot context. */
import type { CollectionPage, EntityId, RevisionId, SnapshotId } from "@scriberig/protocol";
/** Identifies one compatible relation page. */
export interface RelationPageKey {
  snapshotId: SnapshotId;
  entityId: EntityId;
  relation: string;
  cursor?: string;
  revisionId?: RevisionId;
}
/** Caches pages without mixing different snapshots.
 *
 * Public members: `set` and `get`.
 * Private members: the internal page map.
 */
export class RelationCache {
  private readonly values = new Map<string, CollectionPage>();
  /** Stores a page. @param key Page identity. @param page Loaded page. */
  set(key: RelationPageKey, page: CollectionPage): void {
    this.values.set(JSON.stringify(key), page);
  }
  /** Reads a page. @param key Page identity. @returns Loaded page. */
  get(key: RelationPageKey): CollectionPage | undefined {
    return this.values.get(JSON.stringify(key));
  }
}
