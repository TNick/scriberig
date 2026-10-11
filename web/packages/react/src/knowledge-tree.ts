/** @file Binds the headless knowledge tree controller to React. */
import { useCallback, useSyncExternalStore } from "react";
import { useScriberig } from "./provider";
/** Exposes rows and lazy tree actions without rendering markup.
 * @returns Tree lookup and actions.
 */
export function useKnowledgeTree(actions?: {
  /** Refreshes the tree through the host's public client boundary. */
  refresh?: () => void;
  /** Loads the next page for a node through the host's public client boundary. */
  loadMore?: (nodeId: string) => void;
}) {
  const { tree } = useScriberig();
  const subscribe = useCallback(
    (listener: () => void) => tree?.subscribe(listener) ?? (() => undefined),
    [tree],
  );
  const version = useSyncExternalStore(
    subscribe,
    () => tree?.version() ?? 0,
    () => tree?.version() ?? 0,
  );
  return {
    version,
    get: tree?.get.bind(tree),
    expand: tree?.expand.bind(tree),
    collapse: tree?.collapse.bind(tree),
    refresh: actions?.refresh,
    loadMore: actions?.loadMore,
  };
}
