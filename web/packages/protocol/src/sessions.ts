/** @file Defines navigation-oriented session and branch summaries. */
import type { BranchId, EntityId, MigrationId, OperationId, SessionId, SnapshotId } from "./ids";
/** Session summary for navigation. */
export interface SessionSummary {
  sessionId: SessionId;
  title?: string;
  goals: string[];
  decisions: string[];
  tasks: string[];
}
/** Durable activity row returned by a session read. */
export interface SessionTurn {
  id: string;
  ordinal: number;
  hasHumanInput: boolean;
  hasAgentOutput: boolean;
  agentOutput?: string;
  toolInvocation?: string;
  toolResult?: string;
}
/** Compact durable result reference returned for a branch. */
export interface SessionBranchResult {
  id: string;
  references: readonly string[];
}
/** Durable branch node returned by a session read. */
export interface SessionBranch {
  id: BranchId;
  parent?: BranchId;
  children: readonly BranchId[];
  title: string;
  status: "Active" | "Completed" | "Abandoned";
  turns: readonly SessionTurn[];
  result?: SessionBranchResult;
}
/** Compact durable session tree returned by the public protocol. */
export interface SessionReadResponse {
  session: { id: SessionId };
  branches: readonly SessionBranch[];
}
/** Branch summary and purpose. */
export interface BranchSummary {
  branchId: BranchId;
  name: string;
  purpose?: string;
  outcome?: string;
}
/** Stable reference to one durable branch result. */
export interface BranchResultReference {
  branchId: BranchId;
  resultId: string;
  summary?: string;
  handles: readonly string[];
}
/** Bounded descriptor for one session branch. */
export interface BranchDescriptor extends BranchSummary {
  status: "created" | "active" | "completed" | "abandoned";
  parentBranchId?: BranchId;
  childCount: number;
  activityCount: number;
  result?: BranchResultReference;
}
/** One bounded durable activity entry. */
export interface BranchActivity {
  id: string;
  ordinal: number;
  kind: "human" | "model" | "tool" | "child" | "result";
  summary: string;
  handles: readonly string[];
}
/** A cursor-bounded branch activity page. */
export interface BranchActivityPage {
  branchId: BranchId;
  items: readonly BranchActivity[];
  nextCursor?: string;
}
/** A cursor-bounded child branch page. */
export interface BranchChildrenPage {
  branchId: BranchId;
  items: readonly BranchDescriptor[];
  nextCursor?: string;
}
/** Semantic selection attached to an agent request. */
export interface AgentSelection {
  entityId?: EntityId;
  revisionId?: string;
  snapshotId?: string;
  location?: string;
}
/** Shared execution limits accepted by the public agent boundary. */
export interface AgentLimits {
  maxModelSteps?: number;
  maxToolCalls?: number;
  maxBranchDepth?: number;
  maxActiveBranches?: number;
  maxDurationMs?: number;
}
/** Provider-neutral agent start request. */
export interface AgentStartRequest {
  sessionId: SessionId;
  prompt: string;
  selection?: AgentSelection;
  limits?: AgentLimits;
}
/** Provider-neutral agent continuation request. */
export interface AgentContinueRequest {
  sessionId: SessionId;
  branchId: BranchId;
  prompt: string;
  selection?: AgentSelection;
  limits?: AgentLimits;
}
/** Accepted agent operation and durable handles. */
export interface AgentAcceptedResponse {
  operationId: OperationId;
  sessionId: SessionId;
  branchId?: BranchId;
}
/** Structured terminal agent result. */
export interface AgentResult {
  sessionId: SessionId;
  rootBranchId: BranchId;
  branchResultId: string;
  finalOutput: string;
  handles: readonly string[];
  candidateChanges: readonly CandidateChangeReference[];
}
/** Candidate change requiring normal migration review and validation. */
export interface CandidateChangeReference {
  migrationId?: MigrationId;
  entityId?: EntityId;
  candidateSnapshotId?: SnapshotId;
  validationRefs: readonly string[];
}
/** Structured agent failure safe for browser presentation. */
export interface AgentFailure {
  category: string;
  message: string;
  operationId?: OperationId;
}
/** Reference to a tool result or resulting semantic object. */
export interface SessionReference {
  kind: "tool" | "result" | "migration" | "object";
  id: string | MigrationId | EntityId;
}
