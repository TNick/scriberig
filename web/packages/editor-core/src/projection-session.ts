/** @file Defines semantic projection coordination state. */
import type { Selection } from "./selection";
export interface ProjectionSession {
  selection: Selection;
  projectionId: string;
  baseSnapshot?: string;
  candidateSnapshot?: string;
  stale: boolean;
  conflict: boolean;
}
