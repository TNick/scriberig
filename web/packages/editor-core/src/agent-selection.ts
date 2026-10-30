/** @file Maps semantic editor selection into provider-neutral agent scope. */
import type { AgentSelection, EntityId } from "@scriberig/protocol";
import type { Selection } from "./selection";

/** Creates agent scope from exact semantic identity without view-only anchors.
 * @param selection Current editor selection, when present.
 * @param projectId Active project identity.
 * @param rootId Semantic root of the active project workspace.
 * @returns Agent scope with exact entity/snapshot and owning project/root, or undefined.
 */
export function agentSelectionFromSemanticSelection(
  selection: Selection | undefined,
  projectId: string | undefined,
  rootId: EntityId | undefined,
): AgentSelection | undefined {
  if (selection === undefined) return undefined;
  return {
    entityId: selection.entityId,
    snapshotId: selection.snapshotId,
    ...(projectId === undefined ? {} : { projectId }),
    ...(rootId === undefined ? {} : { rootId }),
    ...(selection.revisionId === undefined ? {} : { revisionId: selection.revisionId }),
  };
}
