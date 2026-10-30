/** @file Normalizes bounded source-candidate review and apply results. */
import type {
  ProjectCandidateApplyResult,
  ProjectCandidateEffectSummary,
  ProjectCandidateReview,
} from "@scriberig/protocol";

/** Normalizes a bounded candidate review response.
 * @param value Untyped response returned by the project transport.
 * @returns Candidate identity, validation, and bounded effect summaries.
 * @throws When required values are malformed or source bytes are included.
 */
export function normalizeProjectCandidateReview(value: unknown): ProjectCandidateReview {
  const response = record(value, "candidate review");
  if (!Array.isArray(response.effects) || !Array.isArray(response.diagnostics)) {
    throw new TypeError("candidate review effects and diagnostics must be arrays");
  }
  if (field(response, "source_bytes_included", "sourceBytesIncluded") !== false) {
    throw new TypeError("candidate review must not contain source bytes");
  }
  const validation = string(response.validation, "validation");
  if (validation !== "pending" && validation !== "passed" && validation !== "failed") {
    throw new TypeError("candidate review validation state is invalid");
  }
  const semanticOperation = optionalString(
    field(response, "semantic_operation", "semanticOperation"),
    "semanticOperation",
  );
  const semanticTarget = optionalString(
    field(response, "semantic_target", "semanticTarget"),
    "semanticTarget",
  );
  const rawSemanticChanges = field(response, "semantic_changes", "semanticChanges");
  if (!Array.isArray(rawSemanticChanges)) {
    throw new TypeError("candidate semantic changes must be an array");
  }
  const semanticChanges = rawSemanticChanges.map(normalizeSemanticChange);
  return {
    candidate: string(response.candidate, "candidate"),
    project: string(response.project, "project"),
    base: string(response.base, "base"),
    entity: string(response.entity, "entity"),
    ...(semanticOperation === undefined ? {} : { semanticOperation }),
    ...(semanticTarget === undefined ? {} : { semanticTarget }),
    semanticChanges,
    semanticChangesOmitted: number(
      field(response, "semantic_changes_omitted", "semanticChangesOmitted"),
      "semanticChangesOmitted",
    ),
    path: string(response.path, "path"),
    pathTruncated: boolean(field(response, "path_truncated", "pathTruncated"), "pathTruncated"),
    validation,
    diagnostics: response.diagnostics.map((item) => string(item, "diagnostic")),
    effects: response.effects.map(normalizeEffect),
    effectCount: number(field(response, "effect_count", "effectCount"), "effectCount"),
    effectsOmitted: number(field(response, "effects_omitted", "effectsOmitted"), "effectsOmitted"),
    sourceBytesIncluded: false,
  };
}

/** Normalizes one bounded semantic value diff.
 * @param value Untyped change returned by the project transport.
 * @returns Safe target, property, and before/after previews.
 * @throws When one required field is malformed.
 */
function normalizeSemanticChange(
  value: unknown,
): ProjectCandidateReview["semanticChanges"][number] {
  const change = record(value, "candidate semantic change");
  return {
    target: string(change.target, "semantic target"),
    property: string(change.property, "semantic property"),
    before: string(change.before, "semantic before value"),
    beforeTruncated: boolean(
      field(change, "before_truncated", "beforeTruncated"),
      "beforeTruncated",
    ),
    after: string(change.after, "semantic after value"),
    afterTruncated: boolean(field(change, "after_truncated", "afterTruncated"), "afterTruncated"),
  };
}

/** Validates an optional semantic candidate field.
 * @param value Untyped field returned by the server.
 * @param name Field name for diagnostics.
 * @returns String when present, otherwise undefined.
 * @throws When a present field is not a string.
 */
function optionalString(value: unknown, name: string): string | undefined {
  if (value === undefined || value === null) return undefined;
  return string(value, name);
}

/** Normalizes the result of applying a candidate.
 * @param value Untyped response returned by the project transport.
 * @returns Exact candidate materialization receipt.
 * @throws When required apply result fields are malformed.
 */
export function normalizeProjectCandidateApplyResult(value: unknown): ProjectCandidateApplyResult {
  const response = record(value, "candidate apply result");
  if (!Array.isArray(response.paths)) throw new TypeError("candidate paths must be an array");
  return {
    candidate: string(response.candidate, "candidate"),
    base: string(response.base, "base"),
    successor: string(response.successor, "successor"),
    path: string(response.path, "path"),
    paths: response.paths.map((path) => string(path, "path")),
    materialization: string(response.materialization, "materialization"),
    history: boolean(response.history, "history"),
  };
}

/** Normalizes one bounded source effect descriptor.
 * @param value Untyped effect returned by the project transport.
 * @returns Safe source path and byte-count summary.
 * @throws When an effect field is malformed or its kind is unsupported.
 */
function normalizeEffect(value: unknown): ProjectCandidateEffectSummary {
  const effect = record(value, "candidate effect");
  const kind = string(effect.kind, "effect kind");
  if (kind !== "create" && kind !== "update" && kind !== "delete") {
    throw new TypeError("candidate effect kind is invalid");
  }
  return {
    path: string(effect.path, "effect path"),
    pathTruncated: boolean(field(effect, "path_truncated", "pathTruncated"), "pathTruncated"),
    kind,
    oldByteLength: number(field(effect, "old_byte_length", "oldByteLength"), "oldByteLength"),
    newByteLength: number(field(effect, "new_byte_length", "newByteLength"), "newByteLength"),
  };
}

/** Reads one field with either Rust snake-case or public camel-case spelling.
 * @param source Record containing the field.
 * @param snake Rust field spelling.
 * @param camel Public field spelling.
 * @returns The present field value.
 */
function field(source: Record<string, unknown>, snake: string, camel: string): unknown {
  return source[snake] ?? source[camel];
}

/** Requires an object response.
 * @param value Candidate value.
 * @param label Diagnostic label.
 * @returns Object record.
 * @throws When the value is not a non-array object.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(label + " must be an object");
  }
  return value as Record<string, unknown>;
}

/** Requires a string response field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated string.
 * @throws When the field is not a string.
 */
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new TypeError("candidate " + name + " must be a string");
  return value;
}

/** Requires a finite numeric response field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated number.
 * @throws When the field is not finite numeric data.
 */
function number(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    throw new TypeError("candidate " + name + " must be a nonnegative number");
  }
  return value;
}

/** Requires a boolean response field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated boolean.
 * @throws When the field is not boolean.
 */
function boolean(value: unknown, name: string): boolean {
  if (typeof value !== "boolean") throw new TypeError("candidate " + name + " must be boolean");
  return value;
}
