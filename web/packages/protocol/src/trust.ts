/** @file Defines transport-neutral trust and provenance inspection values. */

import type { EntityId, OperationId, RevisionId, SnapshotId } from "./ids";

/** Identifies the origin category of semantic knowledge. */
export type TrustSource = "authored" | "observed" | "model_generated" | "imported" | "system";

/** Describes validation freshness and outcome. */
export type ValidationStatus = "current" | "stale" | "failed" | "unknown";

/** Describes the selected semantic object for trust inspection. */
export interface TrustSelection {
  /** Stable entity identity. */
  entityId: EntityId;
  /** Exact snapshot context. */
  snapshotId: SnapshotId;
  /** Optional exact revision context. */
  revisionId?: RevisionId;
}

/** Identifies one immutable provenance input. */
export interface ProvenanceInput extends TrustSelection {
  /** Relationship from the inspected object to this input. */
  relation: string;
}

/** Compact trust information suitable for summary views. */
export interface TrustSummary {
  /** Origin category. */
  source: TrustSource;
  /** Validation state. */
  validation: ValidationStatus;
  /** Optional promotion or approval handle. */
  acceptedVia?: string;
  /** Whether the selected object is restricted. */
  restricted: boolean;
}

/** Detailed provenance and validation information. */
export interface ValidationDiagnostic {
  /** Stable diagnostic code. */
  code: string;
  /** Human-readable diagnostic message. */
  message: string;
  /** Optional semantic location. */
  location?: string;
}

/** Structured validation result without restricted payloads. */
export interface ValidationSummary {
  /** Overall validation state. */
  status: ValidationStatus;
  /** Safe diagnostics. */
  diagnostics: readonly ValidationDiagnostic[];
  /** Optional operation handle. */
  operationId?: OperationId;
}

/** Safe policy evaluation outcome. */
export interface PolicyEvaluationSummary {
  /** Whether the action is allowed. */
  allowed: boolean;
  /** Stable action name. */
  action: string;
  /** Stable semantic scope. */
  scope: string;
  /** Safe denial explanation. */
  reason?: string;
}

/** Approval or promotion record summary. */
export interface ApprovalSummary {
  /** Stable approval handle. */
  handle: string;
  /** Whether approval was recorded. */
  recorded: boolean;
  /** Optional operation handle. */
  operationId?: OperationId;
}

/** Distinguishes candidate promotion outcomes. */
export type PromotionState = "promoted" | "rejected" | "stale" | "conflict";

/** Result of a candidate promotion attempt. */
export interface PromotionResult {
  /** Promotion outcome. */
  state: PromotionState;
  /** Candidate snapshot handle. */
  candidateId?: SnapshotId;
  /** Resulting canonical snapshot handle. */
  promotedSnapshotId?: SnapshotId;
  /** Safe explanation for a non-promoted result. */
  reason?: string;
}

/** Summary of a semantic conflict with resolvable handles. */
export interface ConflictSummary {
  /** Stable conflict handle. */
  handle: string;
  /** Entity involved in the conflict. */
  entityId?: EntityId;
  /** Left side snapshot. */
  leftSnapshotId?: SnapshotId;
  /** Right side snapshot. */
  rightSnapshotId?: SnapshotId;
  /** Optional operation handle. */
  operationId?: OperationId;
}

/** Known state of an external effect. */
export type ExternalEffectState = "applied" | "failed" | "unknown";

/** Summary of an external effect receipt. */
export interface ExternalEffectSummary {
  /** Stable effect or receipt handle. */
  handle: string;
  /** Current effect state. */
  state: ExternalEffectState;
  /** Optional operation handle. */
  operationId?: OperationId;
}

/** Detailed trust information for one selection. */
export interface TrustDetails {
  /** Compact summary. */
  summary: TrustSummary;
  /** Exact provenance edges. */
  provenance: readonly ProvenanceInput[];
  /** Structured validation result. */
  validation?: ValidationSummary;
  /** Human-readable validation diagnostics retained for compact clients. */
  diagnostics: readonly string[];
  /** Optional policy or approval handle. */
  policyHandle?: string;
  /** Optional conflict handle. */
  conflictHandle?: string;
}

/** Structured authorization denial safe for presentation. */
export interface CapabilityDenial {
  /** Requested action. */
  action: string;
  /** Requested semantic scope. */
  scope: string;
  /** Safe explanation. */
  reason: string;
  /** Narrower actions that may be available. */
  alternatives: readonly string[];
  /** Whether approval may grant the request. */
  approvalPossible: boolean;
}

/** Trust events that invalidate an inspector view. */
export type TrustEvent =
  | { type: "candidate_created"; selection: TrustSelection; snapshotId: SnapshotId }
  | { type: "validation_started"; selection: TrustSelection; operationId?: OperationId }
  | { type: "validation_updated"; selection: TrustSelection }
  | { type: "provenance_updated"; selection: TrustSelection }
  | { type: "approval_recorded"; selection: TrustSelection; approval: ApprovalSummary }
  | { type: "conflict_detected"; selection: TrustSelection; conflict: ConflictSummary }
  | { type: "capability_denied"; selection?: TrustSelection; denial: CapabilityDenial }
  | { type: "external_effect_updated"; selection?: TrustSelection; effect: ExternalEffectSummary }
  | { type: "snapshot_promoted"; selection: TrustSelection; result: PromotionResult };
