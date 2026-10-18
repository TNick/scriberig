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
/** Safe, ordered activity record projected from durable session execution data. */
export interface SessionActivityItem {
  id: string;
  session: string;
  sequence: number;
  branch: string;
  parent?: string;
  kind:
    | "HumanInput"
    | "ModelInvocation"
    | "AssistantOutput"
    | "ToolInvocation"
    | "ToolResult"
    | "ChildBranch"
    | "BranchResult";
  state: "Completed" | "Running" | "Failed";
  summary: string;
  durableRef?: string;
  model?: string;
  thinkingLevel?: string;
  toolName?: string;
  toolArguments?: Readonly<Record<string, unknown>>;
  toolResultPreview?: Readonly<Record<string, unknown>>;
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
  activity?: readonly SessionActivityItem[];
  result?: SessionBranchResult;
}
/** Future-invocation model selection shared across session clients. */
export interface SessionExecutionSelection {
  revision: number;
  modelProfile?: string;
  model?: string;
  thinkingLevel?: string;
}
/** Compact durable session tree returned by the public protocol. */
export interface SessionReadResponse {
  session: { id: SessionId };
  branches: readonly SessionBranch[];
  execution: SessionExecutionSelection;
}
/** One normalized message retained for a durable model invocation. */
export interface ModelRequestMessageDetail {
  role: string;
  content: string;
}
/** One trusted tool definition retained for a durable model invocation. */
export interface ModelRequestToolDetail {
  name: string;
  description: string;
  parameters?: string;
}
/** Safe normalized request captured before provider-specific serialization. */
export interface ModelRequestDetailResponse {
  invocation: string;
  model: string;
  thinkingLevel?: string;
  messages: readonly ModelRequestMessageDetail[];
  tools: readonly ModelRequestToolDetail[];
  truncated: boolean;
}
/** Expanded safe detail for one durable tool result. */
export interface ToolResultDetailResponse {
  result: string;
  detail: Readonly<Record<string, unknown>>;
  truncated: boolean;
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
  parentId?: string;
  model?: string;
  thinkingLevel?: string;
  toolName?: string;
  toolArguments?: Readonly<Record<string, unknown>>;
  toolResultPreview?: Readonly<Record<string, unknown>>;
  activityKind?: SessionActivityItem["kind"];
  state?: SessionActivityItem["state"];
}
/** A cursor-bounded branch activity page. */
export interface BranchActivityPage {
  branchId: BranchId;
  sessionId?: SessionId;
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
  projectId?: string;
  rootId?: EntityId;
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
