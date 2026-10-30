/** @file Defines plan creation, evolution, assignment, and lifecycle requests. */

import type { SessionContinuationAnchor } from "./session-plans";

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
  /** Optional upstream model-library assignment. */
  assignment?: SessionPlanAssignmentRequest;
}

/** Request to append ordinary work to a running plan. */
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

/** Request to decompose one plan node into structural children. */
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

/** One request-local work node used when creating a plan. */
export interface SessionPlanNodeDraft {
  /** Stable key used by request-local dependency declarations. */
  key: string;
  /** Parent proposal index, or null to attach to the selected branch anchor. */
  parent_index: number | null;
  /** Human-readable work title. */
  title: string;
  /** Focused worker instruction. */
  purpose: string;
  /** Optional expected result contract. */
  expected_result: SessionPlanExpectedResultRequest | null;
  /** Exact semantic handles selected for this node. */
  semantic_scope: readonly string[];
  /** Optional upstream model-library assignment. */
  assignment: SessionPlanAssignmentRequest | null;
}

/** Request to create a durable plan at an exact session branch head. */
export interface SessionPlanCreateRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Exact durable conversation anchor. */
  anchor: SessionContinuationAnchor;
  /** Durable plan objective. */
  goal: string;
  /** Structured initial work nodes. */
  nodes: readonly SessionPlanNodeDraft[];
  /** Request-local prerequisite/dependent index pairs. */
  dependencies: readonly (readonly [number, number])[];
}

/** Supported shared plan lifecycle operations. */
export type SessionPlanAction = "Start" | "Pause" | "Resume" | "Stop" | "Complete";

/** Request to apply one optimistic plan lifecycle operation. */
export interface SessionPlanLifecycleRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Stable plan identity. */
  plan: string;
  /** Plan mutation revision observed by the caller. */
  expected_revision: number;
  /** Requested shared lifecycle action. */
  action: SessionPlanAction;
}

/** References an existing node or a request-local node proposal in a replan. */
export type SessionPlanNodeReference = { Existing: { id: string } } | { Proposed: { key: string } };

/** Optimistic replan request that preserves completed result identities. */
export interface SessionPlanReplanRequest {
  /** Owning durable session. */
  session: { id: string };
  /** Stable plan identity. */
  plan: string;
  /** Plan mutation revision observed by the caller. */
  expected_revision: number;
  /** Proposed structured additions. */
  additions: readonly SessionPlanNodeDraft[];
  /** New dependency edges between current or proposed nodes. */
  dependencies: readonly (readonly [SessionPlanNodeReference, SessionPlanNodeReference])[];
  /** Existing dependency edges to remove. */
  removed_dependencies: readonly (readonly [string, string])[];
  /** Failed/superseded node and its replacement proposal key. */
  superseded: readonly (readonly [string, string])[];
  /** Assignment changes to existing unstarted nodes. */
  assignment_changes: readonly (readonly [string, SessionPlanAssignmentRequest | null])[];
  /** Completed terminal nodes explicitly retained by the replan. */
  preserved_terminal_nodes: readonly string[];
  /** Safe explanation retained in plan history. */
  reason: string;
}
