/** @file Defines lazy expandable semantic tree state. */
import type { EntityId, ObjectSummary, SnapshotId } from "@scriberig/protocol";
/** One stable semantic tree node. */
export interface TreeNode {
  id: string;
  entityId: EntityId;
  snapshotId: SnapshotId;
  summary?: ObjectSummary;
  expanded: boolean;
  loading: boolean;
  error?: string;
  children: string[];
  nextCursor?: string | undefined;
}
/** Headless tree controller that loads only explicitly expanded nodes.
 *
 * Public members: `setRoot`, `get`, `expand`, and `appendChildren`.
 * Private members: the node map.
 */
export class KnowledgeTree {
  private readonly nodes = new Map<string, TreeNode>();
  private readonly listeners = new Set<() => void>();
  private revision = 0;
  /** Subscribes to tree changes. @param listener Change listener. @returns Unsubscribe function. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  /** Returns the change revision. @returns Monotonic tree revision. */
  version(): number {
    return this.revision;
  }
  /** Notifies React-compatible subscribers of a tree change. */
  private notify(): void {
    this.revision += 1;
    for (const listener of this.listeners) listener();
  }
  /** Adds a root node. @param node Root node. */
  setRoot(node: TreeNode): void {
    this.nodes.set(node.id, node);
    this.notify();
  }
  /** Reads a node. @param id Stable node handle. @returns Node state. */
  get(id: string): TreeNode | undefined {
    return this.nodes.get(id);
  }
  /** Marks a node expanded. @param id Node handle. */
  expand(id: string): void {
    const node = this.nodes.get(id);
    if (node) {
      this.nodes.set(id, { ...node, expanded: true });
      this.notify();
    }
  }
  /** Marks a node collapsed. @param id Node handle. */
  collapse(id: string): void {
    const node = this.nodes.get(id);
    if (node) {
      this.nodes.set(id, { ...node, expanded: false });
      this.notify();
    }
  }

  /** Applies one lazy child page. @param id Parent node. @param children Child nodes. @param nextCursor Continuation cursor. */
  appendChildren(id: string, children: readonly TreeNode[], nextCursor?: string): void {
    const node = this.nodes.get(id);
    if (!node) return;
    for (const child of children) this.nodes.set(child.id, child);
    this.nodes.set(id, {
      ...node,
      loading: false,
      children: [...node.children, ...children.map((child) => child.id)],
      nextCursor,
    });
    this.notify();
  }
}
