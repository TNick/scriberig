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
  /** Adds a root node. @param node Root node. */
  setRoot(node: TreeNode): void {
    this.nodes.set(node.id, node);
  }
  /** Reads a node. @param id Stable node handle. @returns Node state. */
  get(id: string): TreeNode | undefined {
    return this.nodes.get(id);
  }
  /** Marks a node expanded. @param id Node handle. */
  expand(id: string): void {
    const node = this.nodes.get(id);
    if (node) this.nodes.set(id, { ...node, expanded: true });
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
  }
}
