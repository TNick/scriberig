/** @file Normalizes typed semantic plan gate, evidence, and summary views. */

import type {
  SessionPlanEvidenceAssociation,
  SessionPlanEvidenceKind,
  SessionPlanEvidenceReference,
  SessionPlanGate,
  SessionPlanGateCounts,
  SessionPlanGateRequirement,
  SessionPlanNode,
  SessionPlanStatusCounts,
  SessionPlanSummary,
  SessionSemanticReference,
} from "@scriberig/protocol";

/** Normalizes a list of exact stable identity strings.
 * @param value Unknown wire value.
 * @returns Valid string values.
 */
export function normalizeStrings(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

/** Normalizes direct status counters without deriving local rollup semantics.
 * @param value Unknown wire counter record.
 * @returns Bounded counters supplied by the shared semantic view.
 */
export function normalizeStatusCounts(value: unknown): SessionPlanStatusCounts {
  const counts = isObject(value) ? value : {};
  return {
    planned: count(counts.planned),
    ready: count(counts.ready),
    running: count(counts.running),
    completed: count(counts.completed),
    failed: count(counts.failed),
    abandoned: count(counts.abandoned),
  };
}

/** Normalizes one gate count group.
 * @param value Unknown wire gate counts.
 * @returns Counts supplied by the semantic view, or `undefined`.
 */
export function normalizeGateCounts(value: unknown): SessionPlanGateCounts | undefined {
  if (!isObject(value)) return undefined;
  return {
    pending: count(value.pending),
    satisfied: count(value.satisfied),
    failed: count(value.failed),
    waived: count(value.waived),
  };
}

/** Converts an unknown numeric count to a finite nonnegative integer.
 * @param value Unknown wire value.
 * @returns Safe count.
 */
function count(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 0;
}

/** Checks the known typed node abandonment values.
 * @param value Unknown wire reason.
 * @returns Whether the value is a registered reason.
 */
export function isAbandonReason(
  value: unknown,
): value is NonNullable<SessionPlanNode["abandonReason"]> {
  return ["Ignored", "Superseded", "Cancelled", "Obsolete"].includes(String(value));
}

/** Normalizes one exact semantic source reference.
 * @param value Unknown wire reference.
 * @returns Exact source handle or `undefined` for malformed input.
 */
function normalizeSemanticReference(value: unknown): SessionSemanticReference | undefined {
  if (!isObject(value) || typeof value.entity !== "string") return undefined;
  const revision = value.revision;
  const snapshot = value.snapshot;
  return {
    entity: value.entity,
    ...(typeof revision === "string" ? { revision } : {}),
    ...(typeof snapshot === "string" ? { snapshot } : {}),
  };
}

/** Normalizes one evidence category without exposing source payloads.
 * @param value Unknown wire evidence category.
 * @returns Registered category or `Other` wrapper.
 */
function normalizeEvidenceKind(value: unknown): SessionPlanEvidenceKind | undefined {
  const known = [
    "BranchResult",
    "ValidationResult",
    "Decision",
    "HumanApproval",
    "Candidate",
    "Artifact",
    "Failure",
  ] as const;
  if (typeof value === "string" && known.includes(value as (typeof known)[number]))
    return value as SessionPlanEvidenceKind;
  if (isObject(value) && typeof value.Other === "string") return { Other: value.Other };
  return undefined;
}

/** Normalizes one exact evidence reference and its bounded attributes.
 * @param value Unknown wire evidence reference.
 * @returns Safe evidence projection or `undefined`.
 */
export function normalizeEvidenceReference(
  value: unknown,
): SessionPlanEvidenceReference | undefined {
  if (!isObject(value)) return undefined;
  const kind = normalizeEvidenceKind(value.kind);
  const reference = normalizeSemanticReference(value.reference);
  if (kind === undefined || reference === undefined) return undefined;
  const attributes = isObject(value.attributes)
    ? Object.fromEntries(
        Object.entries(value.attributes).filter(
          (entry): entry is [string, string] => typeof entry[1] === "string",
        ),
      )
    : {};
  return { kind, reference, attributes };
}

/** Normalizes one durable gate projection.
 * @param value Unknown wire gate.
 * @returns A valid gate projection or an empty list.
 */
export function normalizeGate(value: unknown): SessionPlanGate[] {
  if (!isObject(value) || typeof value.id !== "string" || typeof value.owner !== "string")
    return [];
  const phase = value.phase;
  const requirement = normalizeGateRequirement(value.requirement);
  if ((phase !== "Entry" && phase !== "Completion") || requirement === undefined) return [];
  const waiverValue = value.waiver;
  const waiver =
    isObject(waiverValue) && typeof waiverValue.reason === "string"
      ? {
          reason: waiverValue.reason,
          ...(typeof waiverValue.actor === "string" ? { actor: waiverValue.actor } : {}),
          ...(typeof waiverValue.operation === "string"
            ? { operation: waiverValue.operation }
            : {}),
        }
      : undefined;
  return [{ id: value.id, owner: value.owner, phase, requirement, ...(waiver ? { waiver } : {}) }];
}

/** Validates one tagged non-executable plan gate requirement.
 * @param value Unknown wire requirement.
 * @returns A typed requirement or `undefined`.
 */
function normalizeGateRequirement(value: unknown): SessionPlanGateRequirement | undefined {
  if (!isObject(value)) return undefined;
  const [tag, raw] = Object.entries(value)[0] ?? [];
  const payload = isObject(raw) ? raw : {};
  if (tag === "EvidencePresent") {
    const kind = normalizeEvidenceKind(payload.kind);
    return kind === undefined ? undefined : { EvidencePresent: { kind } };
  }
  if (tag === "ValidationPassed") {
    const profile = nullableString(payload.profile);
    return {
      ValidationPassed: {
        ...(profile === undefined ? {} : { profile }),
      },
    };
  }
  if (tag === "DecisionResolved" && typeof payload.decision === "string") {
    return { DecisionResolved: { decision: payload.decision } };
  }
  if (tag === "HumanApproval" && typeof payload.policy === "string") {
    const role = nullableString(payload.role);
    return {
      HumanApproval: {
        policy: payload.policy,
        ...(role === undefined ? {} : { role }),
      },
    };
  }
  if (tag === "RequiredChildCompleted" && typeof payload.child === "string") {
    return { RequiredChildCompleted: { child: payload.child } };
  }
  if (tag === "SemanticCondition") {
    const reference = normalizeSemanticReference(payload.reference);
    const predicate = payload.predicate;
    if (reference && (predicate === "EntityExists" || predicate === "ExactRevision")) {
      return { SemanticCondition: { reference, predicate } };
    }
  }
  return undefined;
}

/** Normalizes a nullable string field.
 * @param value Unknown wire value.
 * @returns String, null, or `undefined` when malformed.
 */
function nullableString(value: unknown): string | null | undefined {
  return value === null ? null : typeof value === "string" ? value : undefined;
}

/** Normalizes one exact evidence association.
 * @param value Unknown wire association.
 * @returns A valid association or an empty list.
 */
export function normalizeEvidence(value: unknown): SessionPlanEvidenceAssociation[] {
  if (!isObject(value) || typeof value.id !== "string" || !isObject(value.owner)) return [];
  const [ownerKind, ownerId] = Object.entries(value.owner)[0] ?? [];
  const evidence = normalizeEvidenceReference(value.evidence);
  const role = value.role;
  if (
    (ownerKind !== "Node" && ownerKind !== "Gate") ||
    typeof ownerId !== "string" ||
    evidence === undefined ||
    (role !== "Supports" && role !== "Contradicts" && role !== "Informational")
  )
    return [];
  return [
    {
      id: value.id,
      owner: ownerKind === "Node" ? { Node: ownerId } : { Gate: ownerId },
      role,
      evidence,
      ...(typeof value.note === "string" ? { note: value.note } : {}),
      ...(typeof value.actor === "string" ? { actor: value.actor } : {}),
    },
  ];
}

/** Normalizes one concise summary and its exact source references.
 * @param value Unknown wire summary.
 * @returns A valid summary or an empty list.
 */
export function normalizeSummary(value: unknown): SessionPlanSummary[] {
  if (
    !isObject(value) ||
    typeof value.id !== "string" ||
    typeof value.node !== "string" ||
    typeof value.text !== "string"
  )
    return [];
  const sources = Array.isArray(value.sources)
    ? value.sources.flatMap((item) => {
        const normalized = normalizeSemanticReference(item);
        return normalized === undefined ? [] : [normalized];
      })
    : [];
  return [{ id: value.id, node: value.node, text: value.text, sources }];
}

/** Checks whether a value is a JSON object record.
 * @param value Unknown JSON value.
 * @returns Whether the value is a non-null object record.
 */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
