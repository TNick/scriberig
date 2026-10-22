/** @file Defines durable plan views and typed semantic mutation requests. */

import type { BranchId, SessionId } from "./ids";

/** Durable lifecycle state for a planned work item. */
export type SessionPlanNodeStatus =
  "Planned" | "Ready" | "Running" | "Completed" | "Failed" | "Abandoned";
/** Safe attempt metadata without worker transcript content. */
export interface SessionPlanAttempt {
  branch: BranchId;
  claimGeneration: number;
  claim?: string;
  status: SessionPlanNodeStatus;
  result?: string;
  failure?: SessionPlanEvidenceReference;
}
/** Bounded page of attempts retained for one plan node. */
export interface SessionPlanAttemptsResponse {
  plan: string;
  node: string;
  total: number;
  offset: number;
  attempts: readonly SessionPlanAttempt[];
  nextOffset?: number;
}
/** Selects a bounded page from one plan node's attempt history. */
export interface SessionPlanAttemptsRequest {
  session: { id: SessionId };
  plan: string;
  node: string;
  offset?: number;
  limit?: number;
}
/** Selects bounded gate and evidence pages from one observed plan revision. */
export interface SessionPlanDetailsRequest {
  session: { id: SessionId };
  plan: string;
  expectedRevision: number;
  gateOffset?: number;
  evidenceOffset?: number;
  limit?: number;
}
/** Independently paged gate and evidence details for one plan revision. */
export interface SessionPlanDetailsResponse {
  plan: string;
  revision: number;
  snapshot?: string;
  gateTotal: number;
  gateOffset: number;
  gates: readonly SessionPlanGate[];
  gateNextOffset?: number;
  evidenceTotal: number;
  evidenceOffset: number;
  evidence: readonly SessionPlanEvidenceAssociation[];
  evidenceNextOffset?: number;
}
/** Safe model-library configuration assigned to one plan node. */
export interface SessionModelAssignment {
  providerId: string;
  modelId: string;
  thinkingLevel?: string;
}
/** Durable planned work node located in a session tree. */
export interface SessionPlanNode {
  id: string;
  exactRevision?: string;
  snapshot?: string;
  parent: string;
  title: string;
  purpose: string;
  status: SessionPlanNodeStatus;
  branch?: BranchId;
  result?: string;
  supersededBy?: string;
  expectedResult?: SessionExpectedResult;
  semanticScope: readonly string[];
  assignment?: SessionModelAssignment;
  claim?: SessionWorkerClaim;
  abandonReason?: "Ignored" | "Superseded" | "Cancelled" | "Obsolete";
  lastFailure?: SessionPlanEvidenceReference;
  effectiveStatus?: SessionPlanNodeStatus;
  containsIgnored?: boolean;
  containsAbandoned?: boolean;
  entryGates?: SessionPlanGateCounts;
  completionGates?: SessionPlanGateCounts;
  evidenceCount?: number;
  canRetry?: boolean;
}

/** Exact result contract expected from one plan node. */
export interface SessionExpectedResult {
  kind: string;
  requiresHandle: boolean;
}

/** Exclusive runtime claim for an active plan node. */
export interface SessionWorkerClaim {
  id: string;
  node: string;
  branch: BranchId;
  owner: string;
  generation: number;
  assignment?: SessionModelAssignment;
}
/** Result-based dependency edge between plan nodes. */
export interface SessionPlanDependency {
  prerequisite: string;
  dependent: string;
  fulfilledBy?: string;
}

/** Exact semantic source used by a plan summary, gate, or evidence record. */
export interface SessionSemanticReference {
  entity: string;
  revision?: string;
  snapshot?: string;
}
/** Registered categories of semantic evidence understood by plan gates. */
export type SessionPlanEvidenceKind =
  | "BranchResult"
  | "ValidationResult"
  | "Decision"
  | "HumanApproval"
  | "Candidate"
  | "Artifact"
  | "Failure"
  | { Other: string };
/** Exact evidence reference and safe attributes used by typed requirements. */
export interface SessionPlanEvidenceReference {
  kind: SessionPlanEvidenceKind;
  reference: SessionSemanticReference;
  attributes: Readonly<Record<string, string>>;
}
/** Evidence association owner encoded as a typed protocol reference. */
export type SessionPlanEvidenceOwner = { Node: string } | { Gate: string };
/** Evidence role for a node or requirement. */
export type SessionPlanEvidenceRole = "Supports" | "Contradicts" | "Informational";
/** Durable evidence association without copied source payloads. */
export interface SessionPlanEvidenceAssociation {
  id: string;
  owner: SessionPlanEvidenceOwner;
  role: SessionPlanEvidenceRole;
  evidence: SessionPlanEvidenceReference;
  note?: string;
  actor?: string;
}
/** Lifecycle phase in which a typed plan requirement applies. */
export type SessionPlanGatePhase = "Entry" | "Completion";
/** Supported non-executable predicate for semantic-condition gates. */
export type SessionPlanSemanticPredicate = "EntityExists" | "ExactRevision";
/** Small typed plan-gate requirement vocabulary. */
export type SessionPlanGateRequirement =
  | { EvidencePresent: { kind: SessionPlanEvidenceKind } }
  | { ValidationPassed: { profile?: string | null } }
  | { DecisionResolved: { decision: string } }
  | { HumanApproval: { policy: string; role?: string | null } }
  | {
      SemanticCondition: {
        reference: SessionSemanticReference;
        predicate: SessionPlanSemanticPredicate;
      };
    }
  | { RequiredChildCompleted: { child: string } };
/** Inspectable gate waiver provenance. */
export interface SessionPlanGateWaiver {
  reason: string;
  actor?: string;
  operation?: string;
}
/** Revisioned typed requirement attached to one PlanNode. */
export interface SessionPlanGate {
  id: string;
  owner: string;
  phase: SessionPlanGatePhase;
  requirement: SessionPlanGateRequirement;
  waiver?: SessionPlanGateWaiver;
}
/** Current concise node summary with exact source references. */
export interface SessionPlanSummary {
  id: string;
  node: string;
  text: string;
  sources: readonly SessionSemanticReference[];
}
/** Direct and derived PlanNode status counters. */
export interface SessionPlanStatusCounts {
  planned: number;
  ready: number;
  running: number;
  completed: number;
  failed: number;
  abandoned: number;
}
/** Gate evaluation counts for one lifecycle phase. */
export interface SessionPlanGateCounts {
  pending: number;
  satisfied: number;
  failed: number;
  waived: number;
}
/** Request-local PlanNode draft accepted by shared plan operations. */
export interface SessionPlanNodeDraft {
  key: string;
  parentIndex?: number;
  title: string;
  purpose: string;
  expectedResult?: SessionExpectedResult;
  semanticScope: readonly string[];
  assignment?: SessionModelAssignment;
}
/** Typed semantic mutation accepted by the shared application operation. */
export type SessionPlanMutation =
  | { AddNextStep: { node: SessionPlanNodeDraft } }
  | { Decompose: { parent: string; children: readonly SessionPlanNodeDraft[]; required: boolean } }
  | {
      AddGate: {
        owner: string;
        phase: SessionPlanGatePhase;
        requirement: SessionPlanGateRequirement;
      };
    }
  | { WaiveGate: { gate: string; reason: string } }
  | {
      AttachEvidence: {
        owner: SessionPlanEvidenceOwner;
        role: SessionPlanEvidenceRole;
        evidence: SessionPlanEvidenceReference;
        note?: string;
      };
    }
  | {
      UpdateSummary: {
        node: string;
        text: string;
        sources: readonly SessionSemanticReference[];
      };
    }
  | { RetryFailedNode: { node: string } }
  | "RecoverFailedPlan";
/** Optimistic request for one typed semantic plan mutation. */
export interface SessionPlanMutationRequest {
  session: { id: SessionId };
  plan: string;
  expectedRevision: number;
  mutation: SessionPlanMutation;
}

/** Durable plan lifecycle state. */
export type SessionPlanStatus =
  "Draft" | "Planned" | "Running" | "Paused" | "Completed" | "Failed" | "Cancelled";

/** Exact durable conversation location where a plan was created. */
export type SessionContinuationAnchor =
  | { BranchHead: { branch: BranchId } }
  | { AssistantOutput: { branch: BranchId; turn: string } }
  | { HumanInput: { branch: BranchId; turn: string } }
  | { BranchResult: { branch: BranchId; result: string } };

/** Safe identity and generation of the active plan coordinator. */
export interface SessionCoordinatorClaim {
  id: string;
  operation?: string;
  owner: string;
  generation: number;
}

/** Immutable earlier plan revision. */
export interface SessionPlanRevision {
  revision: number;
  goal: string;
  status: SessionPlanStatus;
  nodes: readonly SessionPlanNode[];
  dependencies: readonly SessionPlanDependency[];
  summaries: readonly SessionPlanSummary[];
  gates: readonly SessionPlanGate[];
  evidence: readonly SessionPlanEvidenceAssociation[];
  reason?: string;
}
/** Plan projection with structural nodes and a distinct dependency DAG. */
export interface SessionPlan {
  id: string;
  revision: number;
  exactRevision?: string;
  snapshot?: string;
  goal: string;
  status: SessionPlanStatus;
  anchor?: SessionContinuationAnchor;
  rootBranch: BranchId;
  nodes: readonly SessionPlanNode[];
  dependencies: readonly SessionPlanDependency[];
  rootNodes: readonly string[];
  statusCounts: SessionPlanStatusCounts;
  gates: readonly SessionPlanGate[];
  evidence: readonly SessionPlanEvidenceAssociation[];
  summaries: readonly SessionPlanSummary[];
  coordinator?: BranchId;
  coordinatorCursor?: string;
  coordinatorClaim?: SessionCoordinatorClaim;
  history: readonly SessionPlanRevision[];
}
