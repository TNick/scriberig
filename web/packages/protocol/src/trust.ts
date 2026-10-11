/** @file Defines transport-neutral trust and provenance inspection values. */

import type { EntityId, RevisionId, SnapshotId } from "./ids";

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
export interface TrustDetails {
  /** Compact summary. */
  summary: TrustSummary;
  /** Exact provenance edges. */
  provenance: readonly ProvenanceInput[];
  /** Human-readable validation diagnostics. */
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
  | { type: "validation_updated"; selection: TrustSelection }
  | { type: "provenance_updated"; selection: TrustSelection }
  | { type: "conflict_detected"; selection: TrustSelection };
