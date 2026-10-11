/** @file Binds the shared headless selection controller to React. */
import { useCallback, useSyncExternalStore } from "react";
import type { Selection } from "@scriberig/editor-core";
import { useScriberig } from "./provider";

/** Shares selection state and actions across editor consumers.
 * @returns Selection and controller actions.
 */
export function useSelection(): {
  selection: Selection | undefined;
  setSelection: (value: Selection) => void;
  clearSelection: () => void;
} {
  const { selection: controller } = useScriberig();
  const subscribe = useCallback(
    (listener: () => void) => controller.subscribe(() => listener()),
    [controller],
  );
  const value = useSyncExternalStore(
    subscribe,
    controller.get.bind(controller),
    controller.get.bind(controller),
  );
  return {
    selection: value,
    setSelection: controller.set.bind(controller),
    clearSelection: controller.clear.bind(controller),
  };
}
