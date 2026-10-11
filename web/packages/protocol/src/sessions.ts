/** @file Defines navigation-oriented session and branch summaries. */
import type { BranchId, EntityId, MigrationId, SessionId } from "./ids";
/** Session summary for navigation. */
export interface SessionSummary {
  sessionId: SessionId;
  title?: string;
  goals: string[];
  decisions: string[];
  tasks: string[];
}
/** Branch summary and purpose. */
export interface BranchSummary {
  branchId: BranchId;
  name: string;
  purpose?: string;
  outcome?: string;
}
/** Reference to a tool result or resulting semantic object. */
export interface SessionReference {
  kind: "tool" | "result" | "migration" | "object";
  id: string | MigrationId | EntityId;
}
