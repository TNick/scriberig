/** @file Normalizes safe model library and session selection responses. */
import type {
  ModelLibraryResponse,
  ModelDescriptor,
  SessionExecutionSelection,
  SessionExecutionChange,
} from "@scriberig/protocol";

/** Normalizes Rust or JavaScript model-library field casing.
 * @param value Unknown wire response.
 * @returns Safe upstream-derived model descriptors.
 * @throws When a present model descriptor or metadata snapshot is malformed.
 */
export function normalizeModelLibrary(value: unknown): ModelLibraryResponse {
  if (typeof value !== "object" || value === null) throw new Error("invalid model library");
  const record = value as Record<string, unknown>;
  return {
    models: Array.isArray(record.models) ? record.models.map(normalizeModel) : [],
  };
}

/** Normalizes one safe model descriptor.
 * @param value Unknown model descriptor.
 * @returns Public descriptor with browser field casing.
 * @throws When required identifiers are missing.
 */
function normalizeModel(value: unknown): ModelDescriptor {
  const record = asRecord(value, "model");
  const metadata = record.metadata_snapshot ?? record.metadataSnapshot;
  const levels = Array.isArray(record.thinking_levels)
    ? record.thinking_levels
    : Array.isArray(record.thinkingLevels)
      ? record.thinkingLevels
      : [];
  return {
    id: requiredString(record.id, "model id"),
    displayName: requiredString(record.display_name ?? record.displayName, "model name"),
    providerId: requiredString(
      record.provider_id ?? record.providerId ?? record.provider_profile ?? record.providerProfile,
      "provider id",
    ),
    modelId: requiredString(
      record.model_id ?? record.modelId ?? record.provider_model_id ?? record.providerModelId,
      "model id",
    ),
    thinkingLevels: levels.map((level) => requiredString(level, "thinking level id")),
    ...(metadata === undefined ? {} : { metadataSnapshot: normalizeMetadataSnapshot(metadata) }),
  };
}

/** Normalizes optional upstream display metadata without synthesizing values.
 * @param value Unknown model metadata snapshot.
 * @returns Validated metadata snapshot.
 * @throws When a present snapshot is malformed.
 */
function normalizeMetadataSnapshot(
  value: unknown,
): NonNullable<ModelDescriptor["metadataSnapshot"]> {
  const metadata = asRecord(value, "model metadata");
  const cost = asRecord(metadata.cost, "model cost");
  return {
    schemaVersion: requiredString(
      metadata.schema_version ?? metadata.schemaVersion,
      "model metadata schema version",
    ),
    apiId: requiredString(metadata.api_id ?? metadata.apiId, "model API id"),
    reasoning: requiredBoolean(metadata.reasoning, "model reasoning capability"),
    input: requiredArray(metadata.input, "model input modalities").map((modality) => {
      if (modality !== "text" && modality !== "image") {
        throw new Error("invalid model input modality");
      }
      return modality;
    }),
    cost: {
      input: requiredNumber(cost.input, "model input cost"),
      output: requiredNumber(cost.output, "model output cost"),
      cacheRead: requiredNumber(cost.cache_read ?? cost.cacheRead, "model cache read cost"),
      cacheWrite: requiredNumber(cost.cache_write ?? cost.cacheWrite, "model cache write cost"),
      tiers: (Array.isArray(cost.tiers) ? cost.tiers : []).map((tier) => {
        const item = asRecord(tier, "model cost tier");
        return {
          input: requiredNumber(item.input, "tier input cost"),
          output: requiredNumber(item.output, "tier output cost"),
          cacheRead: requiredNumber(item.cache_read ?? item.cacheRead, "tier cache read cost"),
          cacheWrite: requiredNumber(item.cache_write ?? item.cacheWrite, "tier cache write cost"),
          inputTokensAbove: requiredNumber(
            item.input_tokens_above ?? item.inputTokensAbove,
            "tier input token threshold",
          ),
        };
      }),
    },
    costConfigured: requiredBoolean(
      metadata.cost_configured ?? metadata.costConfigured,
      "model cost configured flag",
    ),
    contextWindow: requiredNumber(
      metadata.context_window ?? metadata.contextWindow,
      "model context window",
    ),
    maxTokens: requiredNumber(metadata.max_tokens ?? metadata.maxTokens, "model output limit"),
  };
}

/** Normalizes the shared session selection returned by a mutation.
 * @param value Unknown selection response.
 * @returns Validated selection with older-server defaults.
 * @throws When a malformed selection cannot be normalized.
 */
export function normalizeExecutionSelection(value: unknown): SessionExecutionSelection {
  const record =
    typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
  const history = Array.isArray(record.history)
    ? record.history.flatMap((value) => {
        if (typeof value !== "object" || value === null) return [];
        const change = value as Record<string, unknown>;
        const kind = change.kind;
        if (
          (kind !== "ModelSelectionChanged" && kind !== "ThinkingLevelChanged") ||
          typeof change.revision !== "number" ||
          typeof change.summary !== "string"
        )
          return [];
        return [
          {
            revision: change.revision,
            kind: kind as SessionExecutionChange["kind"],
            summary: change.summary,
            ...optionalString(change.provider_id ?? change.providerId, "providerId"),
            ...optionalString(change.model_id ?? change.modelId, "modelId"),
            ...optionalString(change.thinking_level ?? change.thinkingLevel, "thinkingLevel"),
          },
        ];
      })
    : undefined;
  return {
    revision: typeof record.revision === "number" ? record.revision : 0,
    ...optionalString(record.provider_id ?? record.providerId, "providerId"),
    ...optionalString(record.model_id ?? record.modelId, "modelId"),
    ...optionalString(record.thinking_level ?? record.thinkingLevel, "thinkingLevel"),
    ...(history === undefined ? {} : { history }),
  };
}

/** Reads an object value or throws a focused normalization error.
 * @param value Unknown value.
 * @param label Field description for diagnostics.
 * @returns Object fields.
 * @throws When value is not an object.
 */
function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null) throw new Error(`invalid ${label}`);
  return value as Record<string, unknown>;
}

/** Reads a required string field.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The string field.
 * @throws When value is not a string.
 */
function requiredString(value: unknown, label: string): string {
  if (typeof value !== "string") throw new Error(`missing ${label}`);
  return value;
}

/** Reads a required finite numeric field.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The numeric field.
 * @throws When value is not a finite number.
 */
function requiredNumber(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`missing ${label}`);
  }
  return value;
}

/** Reads a required boolean field.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The boolean field.
 * @throws When value is not a boolean.
 */
function requiredBoolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw new Error(`missing ${label}`);
  return value;
}

/** Reads a required array field.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The array contents.
 * @throws When value is not an array.
 */
function requiredArray(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`missing ${label}`);
  return value;
}

/** Returns one optional string property when present.
 * @param value Unknown field.
 * @param key Browser-facing property name.
 * @returns Empty or one optional string property.
 */
function optionalString(value: unknown, key: string): Record<string, string> {
  return typeof value === "string" ? { [key]: value } : {};
}
