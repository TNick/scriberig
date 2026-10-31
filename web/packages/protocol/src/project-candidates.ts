/** @file Defines bounded review and apply results for durable project candidates. */

/** One source effect summarized without returning source byte contents.
 * Public members: path, pathTruncated, kind, oldByteLength, and newByteLength.
 * Private members: none.
 */
export interface ProjectCandidateEffectSummary {
  /** Bounded project-relative source path preview. */
  path: string;
  /** Whether path text was truncated for the response. */
  pathTruncated: boolean;
  /** Type of source effect. */
  kind: "create" | "update" | "delete";
  /** Number of bytes in the prior source member. */
  oldByteLength: number;
  /** Number of bytes in the proposed source member. */
  newByteLength: number;
}

/** One bounded typed semantic before/after value in a candidate review.
 * Public members: target, property, before, beforeTruncated, after, and afterTruncated.
 * Private members: none.
 */
export interface ProjectCandidateSemanticChange {
  /** Semantic entity whose property changed. */
  target: string;
  /** Stable domain property key. */
  property: string;
  /** JSON-encoded prior semantic value preview. */
  before: string;
  /** Whether the prior value preview was truncated. */
  beforeTruncated: boolean;
  /** JSON-encoded proposed semantic value preview. */
  after: string;
  /** Whether the proposed value preview was truncated. */
  afterTruncated: boolean;
}

/** Bounded durable candidate state suitable for shared review surfaces.
 * Public members: candidate, project, base, entity, path, pathTruncated,
 * semanticOperation, semanticTarget, semanticChanges, semanticChangesOmitted, validation, diagnostics, effects,
 * effectCount, effectsOmitted, and sourceBytesIncluded.
 * Private members: none.
 */
export interface ProjectCandidateReview {
  /** Stable durable candidate identity. */
  candidate: string;
  /** Stable project identity. */
  project: string;
  /** Exact candidate base snapshot. */
  base: string;
  /** Selected semantic entity. */
  entity: string;
  /** Stable semantic operation that created this candidate, when known. */
  semanticOperation?: string;
  /** Exact semantic entity targeted by the operation, when known. */
  semanticTarget?: string;
  /** Bounded typed semantic before/after values for the candidate. */
  semanticChanges: ProjectCandidateSemanticChange[];
  /** Number of semantic values omitted by the response bound. */
  semanticChangesOmitted: number;
  /** Bounded project-relative source path preview. */
  path: string;
  /** Whether path text was truncated for the response. */
  pathTruncated: boolean;
  /** Candidate validation state. */
  validation: "pending" | "passed" | "failed";
  /** Bounded safe validation diagnostics. */
  diagnostics: string[];
  /** Bounded source effect summaries. */
  effects: ProjectCandidateEffectSummary[];
  /** Total source effect count. */
  effectCount: number;
  /** Number of source effect summaries omitted by the response bound. */
  effectsOmitted: number;
  /** False because review responses never contain source bytes. */
  sourceBytesIncluded: false;
}

/** Result of applying one validated project candidate.
 * Public members: candidate, base, successor, path, paths, materialization,
 * and history.
 * Private members: none.
 */
export interface ProjectCandidateApplyResult {
  /** Candidate that was applied. */
  candidate: string;
  /** Exact base snapshot required by the candidate. */
  base: string;
  /** Snapshot created by successful materialization. */
  successor: string;
  /** Primary changed project-relative path. */
  path: string;
  /** All paths changed by the candidate. */
  paths: string[];
  /** Durable materialization result state. */
  materialization: string;
  /** Whether the canonical project history was updated. */
  history: boolean;
}

/** Describes durable materialization state returned when candidate apply fails. */
export interface ProjectCandidateApplyFailure {
  /** Stable apply failure category. */
  code: string;
  /** Candidate whose external effects were attempted. */
  candidate: string;
  /** Aggregate materialization state persisted for this candidate. */
  materialization: string;
  /** Aggregate failure or recovery diagnostic, when available. */
  diagnostic?: string;
  /** Per-path committed, failed, and pending materialization receipts. */
  paths: ProjectCandidateMaterializationPath[];
  /** Total number of durable path receipts available on the server. */
  pathCount: number;
  /** Number of path receipts omitted by the transport bound. */
  pathsOmitted: number;
}

/** Describes the durable materialization receipt for one source path. */
export interface ProjectCandidateMaterializationPath {
  /** Exact project-relative source path. */
  path: string;
  /** Durable state of this path's materialization. */
  state: string;
  /** Failure or recovery diagnostic for this path, when available. */
  diagnostic?: string;
}
