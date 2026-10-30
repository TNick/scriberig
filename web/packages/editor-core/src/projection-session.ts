/** @file Defines semantic projection coordination state. */
import type { ProjectionStyleResolveResponse } from "@scriberig/protocol";
import type { Selection } from "./selection";

/** Holds semantic navigation and exact render-style evidence for one projection.
 *
 * Public members: the semantic selection, projection identity, snapshots, conflict state, and resolved style.
 * Private members: none.
 */
export interface ProjectionSession {
  /** Exact semantic object selection rendered in this session. */
  selection: Selection;
  /** Stable projection handler identity. */
  projectionId: string;
  /** Snapshot used to render the current projection. */
  baseSnapshot?: string;
  /** Snapshot containing the current editable candidate. */
  candidateSnapshot?: string;
  /** Whether newer canonical state has made the session stale. */
  stale: boolean;
  /** Whether reconciliation reported an unresolved conflict. */
  conflict: boolean;
  /** Exact style, provenance, purpose, and fidelity used for rendering. */
  resolvedStyle?: ProjectionStyleResolveResponse;
}

/** Associates shared style resolution with an existing projection session.
 * @param session Current semantic projection session.
 * @param resolvedStyle Exact application-resolved style and fidelity.
 * @returns Session retaining the same selection, snapshots, and conflict state.
 * @throws When the style was resolved for materialization or export.
 */
export function withResolvedProjectionStyle(
  session: ProjectionSession,
  resolvedStyle: ProjectionStyleResolveResponse,
): ProjectionSession {
  if (resolvedStyle.purpose !== "interactiveView") {
    throw new TypeError("projection sessions require an interactive view style");
  }
  return { ...session, resolvedStyle };
}
