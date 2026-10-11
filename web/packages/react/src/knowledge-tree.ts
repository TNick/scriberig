/** @file Binds the headless knowledge tree controller to React. */
import { useCallback, useSyncExternalStore } from "react";
import { useScriberig } from "./provider";
/** Exposes rows and lazy tree actions without rendering markup.
 * @returns Tree lookup and actions.
 */
export function useKnowledgeTree() {
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
    refresh: () => undefined,
    loadMore: tree?.appendChildren.bind(tree),
  };
}
