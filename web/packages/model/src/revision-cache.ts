/** @file Caches immutable protocol revisions by exact identity. */
import type { RevisionRef } from "@scriberig/protocol";
/** Stores immutable revision summaries without mutable entity keys.
 *
 * Public members: `set`, `get`, and `has`.
 * Private members: the internal revision map.
 */
export class RevisionCache {
  private readonly values = new Map<string, RevisionRef>();
  /** Inserts a revision idempotently. @param revision Revision to retain. */
  set(revision: RevisionRef): void {
    this.values.set(revision.revisionId, revision);
  }
  /** Reads a cached revision. @param revisionId Exact revision identity. @returns Cached revision. */
  get(revisionId: string): RevisionRef | undefined {
    return this.values.get(revisionId);
  }
  /** Reports whether a revision is loaded. @param revisionId Exact revision identity. @returns Whether loaded. */
  has(revisionId: string): boolean {
    return this.values.has(revisionId);
  }
}
