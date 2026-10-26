/** @file Provides raw semantic knowledge browsing. */
import { useCallback, type ReactElement } from "react";
import { useScriberig } from "@scriberig/react";
import { MuiObjectTree } from "@scriberig/components-mui";
import type { TreeNode } from "@scriberig/editor-core";
import type { AppRuntime } from "../runtime";
/** Renders the shared semantic tree when available. */
/**
 * @returns The rendered RawKnowledgeView view or runtime value.
 * @throws When rendered outside the shared Scriberig provider.
 */
export function RawKnowledgeView(): ReactElement {
  const { selection, tree } = useScriberig();
  const runtime = useScriberig() as unknown as AppRuntime;
  const loadChildren = useCallback(
    async (node: TreeNode) => {
      if (!tree || !tree.beginLoading(node.id)) return;
      const children = runtime.symbols.map((symbol) => ({
        id: symbol.entityId,
        entityId: symbol.entityId as never,
        snapshotId: symbol.snapshotId as never,
        summary: {
          ref: {
            entityId: symbol.entityId as never,
            snapshotId: symbol.snapshotId as never,
          },
          typeId: symbol.kind as never,
          label: `${symbol.name} · ${symbol.kind}`,
        },
        expanded: false,
        loading: false,
        children: [],
      }));
      tree.appendChildren(node.id, children, undefined);
    },
    [runtime.symbols, tree],
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
