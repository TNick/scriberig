/** @file Defines typed plan projections and mutations for session clients. */
import type { BranchId } from "./ids";

/** Durable lifecycle state for a planned work item. */
export type SessionPlanNodeStatus =
  "Planned" | "Ready" | "Running" | "Completed" | "Failed" | "Abandoned";

/** Safe model-library configuration assigned to one plan node. */
export interface SessionModelAssignment {
  providerId: string;
  modelId: string;
  thinkingLevel?: string;
}

/** Counts direct plan nodes by their lifecycle state. */
export interface SessionPlanStatusCounts {
  planned: number;
  ready: number;
  running: number;
  completed: number;
  failed: number;
  abandoned: number;
}

/** Counts evaluated requirements in one gate phase. */
export interface SessionPlanGateCounts {
  pending: number;
  satisfied: number;
  failed: number;
  waived: number;
}

/** Exact result contract expected from one plan node. */
export interface SessionExpectedResult {
  kind: string;
  requiresHandle: boolean;
}

/** Identifies an exact semantic revision or snapshot. */
export interface SessionSemanticReference {
  entity: string;
  revision?: string;
  snapshot?: string;
}

/** Typed category of evidence attached to a plan. */
export type SessionPlanEvidenceKind =
  | "BranchResult"
  | "ValidationResult"
  | "Decision"
  | "HumanApproval"
  | "Candidate"
  | "Artifact"
  | "Failure"
  | { Other: string };

/** Exact evidence reference and safe typed facts. */
export interface SessionPlanEvidenceReference {
  kind: SessionPlanEvidenceKind;
  reference: SessionSemanticReference;
  attributes: Readonly<Record<string, string>>;
}

/** Classifies how evidence affects its owner. */
export type SessionPlanEvidenceRole = "Supports" | "Contradicts" | "Informational";

/** Identifies a plan node or gate that owns evidence. */
export type SessionPlanEvidenceOwner = { Node: string } | { Gate: string };

/** Associates exact evidence with one node or gate. */
export interface SessionPlanEvidenceAssociation {
  id: string;
  owner: SessionPlanEvidenceOwner;
  role: SessionPlanEvidenceRole;
  evidence: SessionPlanEvidenceReference;
  note?: string;
  actor?: string;
}

/** Selects when a typed gate is required. */
export type SessionPlanGatePhase = "Entry" | "Completion";

/** Selects a supported, non-executable semantic predicate. */
export type SessionPlanSemanticPredicate = "EntityExists" | "ExactRevision";

/** Describes one supported plan gate requirement. */
export type SessionPlanGateRequirement =
  | { EvidencePresent: { kind: SessionPlanEvidenceKind } }
  | { ValidationPassed: { profile?: string } }
  | { DecisionResolved: { decision: string } }
  | { HumanApproval: { policy: string; role?: string } }
  | {
      SemanticCondition: {
        reference: SessionSemanticReference;
        predicate: SessionPlanSemanticPredicate;
      };
    }
  | { RequiredChildCompleted: { child: string } };

/** Retains explicit gate-waiver provenance. */
export interface SessionPlanGateWaiver {
  reason: string;
  actor?: string;
  operation?: string;
}

/** Projects one independent typed lifecycle gate. */
export interface SessionPlanGate {
  id: string;
  owner: string;
  phase: SessionPlanGatePhase;
  requirement: SessionPlanGateRequirement;
  waiver?: SessionPlanGateWaiver;
}

/** Concise node summary with exact source references. */
export interface SessionPlanSummary {
  id: string;
  node: string;
  text: string;
  sources: readonly SessionSemanticReference[];
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

/** Durable planned work node with shared bounded-view fields. */
export interface SessionPlanNode {
  id: string;
  parent: string;
  title: string;
  purpose: string;
  status: SessionPlanNodeStatus;
  exactRevision?: string;
  effectiveStatus?: SessionPlanNodeStatus;
  containsIgnored: boolean;
  containsAbandoned: boolean;
  abandonReason?: "Ignored" | "Superseded" | "Cancelled" | "Obsolete";
  branch?: BranchId;
  result?: string;
  supersededBy?: string;
  expectedResult?: SessionExpectedResult;
  semanticScope: readonly string[];
  assignment?: SessionModelAssignment;
  claim?: SessionWorkerClaim;
  summaryId?: string;
  summaryPreview?: string;
  dependencyBlockers: readonly string[];
  entryGates: SessionPlanGateCounts;
  completionGates: SessionPlanGateCounts;
  evidenceCount: number;
  canRetry: boolean;
  lastFailure?: SessionPlanEvidenceReference;
}

/** Durable lifecycle state for a plan. */
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

/** Result-based dependency edge between plan nodes. */
export interface SessionPlanDependency {
  prerequisite: string;
  dependent: string;
  fulfilledBy?: string;
}

/** Immutable earlier plan projection. */
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

/** Plan projection with structural nodes and a dependency DAG. */
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
  coordinator?: string;
  coordinatorCursor?: string;
  coordinatorClaim?: SessionCoordinatorClaim;
  history: readonly SessionPlanRevision[];
}

/** Typed shared plan mutation for summaries, evidence, and gates. */
export type SessionPlanSemanticMutation =
  | { UpdateSummary: { node: string; text: string; sources: readonly SessionSemanticReference[] } }
  | {
      AttachEvidence: {
        owner: SessionPlanEvidenceOwner;
        role: SessionPlanEvidenceRole;
        evidence: SessionPlanEvidenceReference;
        note?: string;
      };
    }
  | {
      AddGate: {
        owner: string;
        phase: SessionPlanGatePhase;
        requirement: SessionPlanGateRequirement;
      };
    }
  | { WaiveGate: { gate: string; reason: string } };

/** Applies one typed plan mutation at an observed plan revision. */
export interface SessionPlanSemanticMutationRequest {
  session: { id: string };
  plan: string;
  expected_revision: number;
  mutation: SessionPlanSemanticMutation;
}

/** Proposed structural child work item for plan evolution. */
export interface SessionPlanChildDraft {
  /** Human-readable work title. */
  title: string;
  /** Focused work instruction. */
  purpose: string;
  /** Provider-neutral result contract, when required. */
  expected_result?: SessionPlanExpectedResultRequest;
  /** Semantic handles selected for the work. */
  semantic_scope: readonly string[];
  /** Optional upstream model-library assignment in protocol field casing. */
  assignment?: SessionPlanAssignmentRequest;
}

/** Provider-neutral result requirements in transport field casing. */
export interface SessionPlanExpectedResultRequest {
  /** Result category. */
  kind: string;
  /** Whether completion must reference a durable result handle. */
  requires_handle: boolean;
}

/** Safe model-library assignment in transport field casing. */
export interface SessionPlanAssignmentRequest {
  /** Upstream provider identity. */
  provider_id: string;
  /** Provider-scoped model identity. */
  model_id: string;
  /** Optional advertised thinking level. */
  thinking_level?: string;
}

/** Request to append ordinary work to a running plan.
 * @param session Owning durable session.
 * @param plan Stable plan identity.
 * @param expected_revision Plan mutation revision observed by the caller.
 * @param title Human-readable work title.
 * @param purpose Focused work instruction.
 * @param expected_result Optional provider-neutral result contract.
 * @param semantic_scope Semantic handles selected for this work.
 * @param assignment Optional upstream model-library assignment.
 */
export interface SessionPlanNextStepRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Stable plan identity. */
  plan: string;
  /** Plan mutation revision observed by the caller. */
  expected_revision: number;
  /** Human-readable work title. */
  title: string;
  /** Focused work instruction. */
  purpose: string;
  /** Optional provider-neutral result contract. */
  expected_result?: SessionPlanExpectedResultRequest;
  /** Semantic handles selected for this work. */
  semantic_scope: readonly string[];
  /** Optional upstream model-library assignment. */
  assignment?: SessionPlanAssignmentRequest;
}

/** Request to decompose one plan node into structural children.
 * @param session Owning durable session.
 * @param plan Stable plan identity.
 * @param parent Stable identity of the node to decompose.
 * @param expected_revision Plan mutation revision observed by the caller.
 * @param children Proposed structural child work items.
 * @param required Whether each child is required for parent completion.
 */
export interface SessionPlanDecomposeRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Stable plan identity. */
  plan: string;
  /** Stable identity of the node to decompose. */
  parent: string;
  /** Plan mutation revision observed by the caller. */
  expected_revision: number;
  /** Proposed structural child work items. */
  children: readonly SessionPlanChildDraft[];
  /** Whether each child is required for parent completion. */
  required: boolean;
}

/** Request to update or clear one node's model-library assignment. */
export interface SessionPlanAssignmentMutationRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Stable plan identity. */
  plan: string;
  /** Stable node identity. */
  node: string;
  /** Plan mutation revision observed by the caller. */
  expected_revision: number;
  /** New safe upstream assignment, or null to inherit defaults. */
  assignment: SessionPlanAssignmentRequest | null;
}

/** Reads a bounded page of node attempts at an exact semantic snapshot. */
export interface SessionPlanAttemptHistoryRequest {
  session: { id: string };
  plan: string;
  node: string;
  snapshot: string;
  offset: number;
  limit: number;
}

/** Reads a bounded page of node gates and evidence at an exact snapshot. */
export interface SessionPlanNodeDetailsRequest {
  session: { id: string };
  plan: string;
  node: string;
  snapshot: string;
  offset: number;
  limit: number;
}

/** One bounded page of gates and their node evidence. */
export interface SessionPlanNodeDetailsPage {
  offset: number;
  gates: readonly SessionPlanGate[];
  evidence: readonly SessionPlanEvidenceAssociation[];
}

/** One durable worker attempt with exact semantic identities. */
export interface SessionPlanAttempt {
  id: string;
  exactRevision: string;
  node: string;
  branch: string;
  claimGeneration: number;
  claim?: string;
  status: SessionPlanNodeStatus;
  result?: string;
  failure?: SessionPlanEvidenceReference;
}

/** One bounded page of a node's attempt history. */
export interface SessionPlanAttemptHistoryPage {
  offset: number;
  attempts: readonly SessionPlanAttempt[];
}
