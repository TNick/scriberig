/** @file Binds editor-core command discovery to React selection state. */
import { useMemo } from "react";
import { discoverCommands } from "@scriberig/editor-core";
import { useScriberig } from "./provider";
import { useSelection } from "./use-selection";
/** Returns commands applicable to the shared selection and capability set.
 * @param capabilities Available capability identifiers.
 * @returns Discovered command descriptors.
 */
export function useCommands(capabilities: ReadonlySet<string> = new Set()) {
  const { commands } = useScriberig();
  const { selection } = useSelection();
  return useMemo(
    () => discoverCommands(commands, selection, capabilities),
    [commands, selection, capabilities],
  );
}
