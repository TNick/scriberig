/** @file Provides raw semantic knowledge browsing. */
import { useCallback, type ReactElement } from "react";
import { useScriberig } from "@scriberig/react";
import { MuiObjectTree } from "@scriberig/components-mui";
import type { TreeNode } from "@scriberig/editor-core";
/** Renders the shared semantic tree when available. */
/**
 * @returns The rendered RawKnowledgeView view or runtime value.
 * @throws When rendered outside the shared Scriberig provider.
 */
export function RawKnowledgeView(): ReactElement {
  const { client, selection, tree } = useScriberig();
  const loadChildren = useCallback(
    async (node: TreeNode) => {
      if (!tree || !tree.beginLoading(node.id)) return;
      try {
        const result = await client.list({
          entityId: node.entityId,
          snapshot: { kind: "exact", snapshotId: node.snapshotId },
          relation: "children",
        });
        if (!("items" in result)) throw new Error("server returned an invalid list response");
        tree.appendChildren(
          node.id,
          result.items.map((summary) => ({
            id: summary.ref.entityId,
            entityId: summary.ref.entityId,
            snapshotId: summary.ref.snapshotId,
            summary,
            expanded: false,
            loading: false,
            children: [],
          })),
          result.nextCursor,
        );
      } catch (error) {
        tree.appendChildren(node.id, [], undefined);
      }
    },
    [client, tree],
  );
  return (
    <section aria-label="Raw Knowledge">
      <h1>Raw Knowledge</h1>
      {tree ? (
        <MuiObjectTree
          tree={tree}
          rootId="root"
          onSelect={(node) => {
            const revisionId = node.summary?.ref.revisionId;
            selection.set({
              entityId: node.entityId,
              snapshotId: node.snapshotId,
              ...(revisionId === undefined ? {} : { revisionId }),
            });
          }}
          onLoadMore={(node) => void loadChildren(node)}
        />
      ) : (
        <p>No knowledge tree is connected.</p>
      )}
    </section>
  );
}
