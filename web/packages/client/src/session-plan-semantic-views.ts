/** @file Validates semantic plan records received from the session protocol. */

import type {
  SessionPlanEvidenceAssociation,
  SessionPlanEvidenceKind,
  SessionPlanEvidenceOwner,
  SessionPlanEvidenceReference,
  SessionPlanEvidenceRole,
  SessionPlanGate,
  SessionPlanGatePhase,
  SessionPlanGateRequirement,
  SessionPlanGateWaiver,
  SessionPlanNodeDetailsPage,
  SessionPlanSemanticPredicate,
  SessionPlanSummary,
  SessionSemanticReference,
} from "@scriberig/protocol";

/** Normalizes typed gates while dropping malformed records.
 * @param value Unknown gate list from the transport.
 * @returns Valid typed gates.
 */
export function normalizePlanGates(value: unknown): SessionPlanGate[] {
  return Array.isArray(value)
    ? value.flatMap((item) => {
        const gate = normalizePlanGate(item);
        return gate === undefined ? [] : [gate];
      })
    : [];
}

/** Normalizes exact evidence associations while dropping malformed records.
 * @param value Unknown evidence list from the transport.
 * @returns Valid exact evidence associations.
 */
export function normalizePlanEvidence(value: unknown): SessionPlanEvidenceAssociation[] {
  return Array.isArray(value)
    ? value.flatMap((item) => {
        const evidence = normalizeEvidenceAssociation(item);
        return evidence === undefined ? [] : [evidence];
      })
    : [];
}

/** Normalizes a bounded exact-snapshot node detail page.
 * @param value Unknown detail page from the transport.
 * @returns Validated page with malformed records removed.
 */
export function normalizePlanNodeDetails(value: unknown): SessionPlanNodeDetailsPage {
  const source = isObject(value) ? value : {};
  const offset = source.offset;
  return {
    offset: typeof offset === "number" && Number.isInteger(offset) && offset >= 0 ? offset : 0,
    gates: normalizePlanGates(source.gates),
    evidence: normalizePlanEvidence(source.evidence),
  };
}

/** Normalizes concise summaries and their exact source references.
 * @param value Unknown summary list from the transport.
 * @returns Valid node summaries.
 */
export function normalizePlanSummaries(value: unknown): SessionPlanSummary[] {
  return Array.isArray(value)
    ? value.flatMap((item) => {
        if (!isObject(item)) return [];
        const id = item.id;
        const node = item.node;
        const text = item.text;
        if (typeof id !== "string" || typeof node !== "string" || typeof text !== "string") {
          return [];
        }
        return [{ id, node, text, sources: normalizeSemanticReferences(item.sources) }];
      })
    : [];
}

/** Normalizes one exact typed evidence reference.
 * @param value Unknown evidence reference.
 * @returns A validated evidence reference or `undefined` when malformed.
 */
export function normalizePlanEvidenceReference(
  value: unknown,
): SessionPlanEvidenceReference | undefined {
  if (!isObject(value)) return undefined;
  const kind = normalizeEvidenceKind(value.kind);
  const reference = normalizeSemanticReference(value.reference);
  if (kind === undefined || reference === undefined) return undefined;
  const attributes = isObject(value.attributes) ? value.attributes : {};
  return {
    kind,
    reference,
    attributes: Object.fromEntries(
      Object.entries(attributes).filter(
        (entry): entry is [string, string] => typeof entry[1] === "string",
      ),
    ),
  };
}

/** Validates one typed gate and optional waiver provenance.
 * @param value Unknown gate record.
 * @returns A validated gate or `undefined` when malformed.
 */
function normalizePlanGate(value: unknown): SessionPlanGate | undefined {
  if (!isObject(value)) return undefined;
  const { id, owner } = value;
  const phase = normalizeGatePhase(value.phase);
  const requirement = normalizeGateRequirement(value.requirement);
  if (
    typeof id !== "string" ||
    typeof owner !== "string" ||
    phase === undefined ||
    requirement === undefined
  ) {
    return undefined;
  }
  const waiver = normalizeWaiver(value.waiver);
  return { id, owner, phase, requirement, ...(waiver === undefined ? {} : { waiver }) };
}

/** Validates one supported non-executable gate condition.
 * @param value Unknown gate requirement.
 * @returns A supported typed requirement or `undefined` when malformed.
 */
function normalizeGateRequirement(value: unknown): SessionPlanGateRequirement | undefined {
  if (!isObject(value)) return undefined;
  if (isObject(value.EvidencePresent)) {
    const kind = normalizeEvidenceKind(value.EvidencePresent.kind);
    return kind === undefined ? undefined : { EvidencePresent: { kind } };
  }
  if (isObject(value.ValidationPassed)) {
    const profile = value.ValidationPassed.profile;
    return {
      ValidationPassed: {
        ...(typeof profile === "string" ? { profile } : {}),
      },
    };
  }
  if (isObject(value.DecisionResolved)) {
    const decision = value.DecisionResolved.decision;
    return typeof decision === "string" ? { DecisionResolved: { decision } } : undefined;
  }
  if (isObject(value.HumanApproval)) {
    const { policy, role } = value.HumanApproval;
    if (typeof policy !== "string") return undefined;
    return {
      HumanApproval: {
        policy,
        ...(typeof role === "string" ? { role } : {}),
      },
    };
  }
  if (isObject(value.SemanticCondition)) {
    const reference = normalizeSemanticReference(value.SemanticCondition.reference);
    const predicate = normalizePredicate(value.SemanticCondition.predicate);
    return reference === undefined || predicate === undefined
      ? undefined
      : { SemanticCondition: { reference, predicate } };
  }
  if (isObject(value.RequiredChildCompleted)) {
    const child = value.RequiredChildCompleted.child;
    return typeof child === "string" ? { RequiredChildCompleted: { child } } : undefined;
  }
  return undefined;
}

/** Validates one evidence association and its typed source.
 * @param value Unknown association record.
 * @returns A valid evidence association or `undefined` when malformed.
 */
function normalizeEvidenceAssociation(value: unknown): SessionPlanEvidenceAssociation | undefined {
  if (!isObject(value)) return undefined;
  const id = value.id;
  const owner = normalizeEvidenceOwner(value.owner);
  const role = normalizeEvidenceRole(value.role);
  const evidence = normalizePlanEvidenceReference(value.evidence);
  if (
    typeof id !== "string" ||
    owner === undefined ||
    role === undefined ||
    evidence === undefined
  ) {
    return undefined;
  }
  return {
    id,
    owner,
    role,
    evidence,
    ...(typeof value.note === "string" ? { note: value.note } : {}),
    ...(typeof value.actor === "string" ? { actor: value.actor } : {}),
  };
}

/** Validates a node-or-gate evidence owner.
 * @param value Unknown owner value.
 * @returns A validated owner or `undefined` when malformed.
 */
function normalizeEvidenceOwner(value: unknown): SessionPlanEvidenceOwner | undefined {
  if (!isObject(value)) return undefined;
  if (typeof value.Node === "string") return { Node: value.Node };
  if (typeof value.Gate === "string") return { Gate: value.Gate };
  return undefined;
}

/** Normalizes one evidence association role.
 * @param value Unknown evidence role.
 * @returns A valid role or `undefined` when malformed.
 */
function normalizeEvidenceRole(value: unknown): SessionPlanEvidenceRole | undefined {
  return value === "Supports" || value === "Contradicts" || value === "Informational"
    ? value
    : undefined;
}

/** Normalizes an optional gate waiver with safe provenance.
 * @param value Unknown waiver record.
 * @returns A validated waiver or `undefined` when absent or malformed.
 */
function normalizeWaiver(value: unknown): SessionPlanGateWaiver | undefined {
  if (!isObject(value) || typeof value.reason !== "string") return undefined;
  return {
    reason: value.reason,
    ...(typeof value.actor === "string" ? { actor: value.actor } : {}),
    ...(typeof value.operation === "string" ? { operation: value.operation } : {}),
  };
}

/** Normalizes supported evidence categories.
 * @param value Unknown evidence category.
 * @returns A known category or `undefined` when malformed.
 */
function normalizeEvidenceKind(value: unknown): SessionPlanEvidenceKind | undefined {
  const kinds = [
    "BranchResult",
    "ValidationResult",
    "Decision",
    "HumanApproval",
    "Candidate",
    "Artifact",
    "Failure",
  ] as const;
  if (typeof value === "string" && kinds.includes(value as (typeof kinds)[number])) {
    return value as SessionPlanEvidenceKind;
  }
  return isObject(value) && typeof value.Other === "string" ? { Other: value.Other } : undefined;
}

/** Normalizes a supported lifecycle gate phase.
 * @param value Unknown gate phase.
 * @returns A valid phase or `undefined` when malformed.
 */
function normalizeGatePhase(value: unknown): SessionPlanGatePhase | undefined {
  return value === "Entry" || value === "Completion" ? value : undefined;
}

/** Normalizes a supported semantic predicate.
 * @param value Unknown predicate.
 * @returns A supported predicate or `undefined` when malformed.
 */
function normalizePredicate(value: unknown): SessionPlanSemanticPredicate | undefined {
  return value === "EntityExists" || value === "ExactRevision" ? value : undefined;
}

/** Normalizes exact semantic references.
 * @param value Unknown reference list.
 * @returns Valid references only.
 */
function normalizeSemanticReferences(value: unknown): SessionSemanticReference[] {
  return Array.isArray(value)
    ? value.flatMap((item) => {
        const reference = normalizeSemanticReference(item);
        return reference === undefined ? [] : [reference];
      })
    : [];
}

/** Validates one entity and its optional exact content location.
 * @param value Unknown semantic reference.
 * @returns A safe reference or `undefined` when malformed.
 */
function normalizeSemanticReference(value: unknown): SessionSemanticReference | undefined {
  if (!isObject(value) || typeof value.entity !== "string") return undefined;
  return {
    entity: value.entity,
    ...(typeof value.revision === "string" ? { revision: value.revision } : {}),
    ...(typeof value.snapshot === "string" ? { snapshot: value.snapshot } : {}),
  };
}

/** Narrows one unknown JSON object.
 * @param value Candidate input.
 * @returns Whether the input is a non-array object.
 */
function isObject(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
