/** @file Normalizes safe model library and session selection responses. */
import type {
  ModelLibraryResponse,
  ModelDescriptor,
  ProviderCredentialReadiness,
  ProviderReadiness,
  SessionExecutionSelection,
  SessionExecutionChange,
} from "@scriberig/protocol";

const SAFE_MODEL_SNAPSHOT_SCHEMA = "rpi-safe-model-snapshot-v1";

/** Normalizes the versioned Rust model-library wire contract.
 * Unknown fields are discarded; required wire fields are validated strictly.
 * @param value Unknown wire response.
 * @returns Safe upstream-derived model descriptors.
 * @throws When a model descriptor or metadata snapshot is malformed.
 */
export function normalizeModelLibrary(value: unknown): ModelLibraryResponse {
  if (typeof value !== "object" || value === null) throw new Error("invalid model library");
  const record = value as Record<string, unknown>;
  const models = requiredArray(record.models, "models").map(normalizeModel);
  const modelIds = new Set(models.map((model) => model.id));
  if (modelIds.size !== models.length) throw new Error("duplicate model row id");
  const readiness = record.provider_readiness;
  const providerReadiness =
    readiness === undefined
      ? undefined
      : requiredArray(readiness, "provider readiness").map(normalizeProviderReadiness);
  if (providerReadiness) {
    const readinessIds = new Set<string>();
    for (const item of providerReadiness) {
      const rowId = modelRowId(item.providerId, item.modelId);
      if (!modelIds.has(rowId)) throw new Error("orphan provider readiness row");
      if (readinessIds.has(rowId)) throw new Error("duplicate provider readiness row");
      readinessIds.add(rowId);
    }
  }
  return {
    models,
    ...(providerReadiness === undefined ? {} : { providerReadiness }),
  };
}

/** Normalizes one host-reported credential readiness row.
 * @param value Unknown readiness row.
 * @returns Credential-safe provider readiness.
 * @throws When an identifier or readiness category is invalid.
 */
function normalizeProviderReadiness(value: unknown): ProviderReadiness {
  const record = asRecord(value, "provider readiness");
  const credential = requiredString(record.credential, "credential readiness");
  const supported: readonly ProviderCredentialReadiness[] = [
    "not_required",
    "available",
    "missing",
    "denied",
    "unsupported",
    "catalog_header",
  ];
  if (!supported.includes(credential as ProviderCredentialReadiness)) {
    throw new Error("invalid credential readiness");
  }
  return {
    providerId: requiredString(record.provider_id, "provider id"),
    modelId: requiredString(record.model_id, "model id"),
    credential: credential as ProviderCredentialReadiness,
  };
}

/** Normalizes one safe model descriptor.
 * @param value Unknown model descriptor.
 * @returns Public descriptor with browser field casing.
 * @throws When required identifiers are missing.
 */
function normalizeModel(value: unknown): ModelDescriptor {
  const record = asRecord(value, "model");
  const metadata = record.metadata_snapshot;
  const levels = requiredArray(record.thinking_levels, "thinking levels");
  const providerId = requiredString(record.provider_id, "provider id");
  const modelId = requiredString(record.provider_model_id, "model id");
  const id = requiredString(record.id, "model row id");
  if (id !== modelRowId(providerId, modelId)) {
    throw new Error("model row id does not match provider/model identity");
  }
  return {
    id,
    displayName: requiredString(record.display_name, "model name"),
    providerId,
    modelId,
    thinkingLevels: levels.map((level) => requiredString(level, "thinking level id")),
    metadataSnapshot: normalizeMetadataSnapshot(metadata),
  };
}

/** Builds the Rust protocol's UTF-8 byte-length-prefixed model row id.
 * @param providerId Exact upstream provider identity.
 * @param modelId Provider-scoped upstream model identity.
 * @returns The canonical presentation id for the provider/model pair.
 */
function modelRowId(providerId: string, modelId: string): string {
  const providerLength = utf8ByteLength(providerId);
  const modelLength = utf8ByteLength(modelId);
  return `rpi1:${providerLength}:${providerId}${modelLength}:${modelId}`;
}

/** Counts UTF-8 bytes without allocating an encoded copy.
 * @param value A JavaScript string expected to represent a Rust string.
 * @returns The string's UTF-8 byte length.
 * @throws When the string contains an unpaired UTF-16 surrogate.
 */
function utf8ByteLength(value: string): number {
  let length = 0;
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined || (codePoint >= 0xd800 && codePoint <= 0xdfff)) {
      throw new Error("invalid unicode model identity");
    }
    if (codePoint <= 0x7f) length += 1;
    else if (codePoint <= 0x7ff) length += 2;
    else if (codePoint <= 0xffff) length += 3;
    else length += 4;
  }
  return length;
}

/** Normalizes required upstream display metadata without synthesizing values.
 * @param value Unknown model metadata snapshot.
 * @returns Validated metadata snapshot.
 * @throws When a required snapshot field is missing or malformed.
 */
function normalizeMetadataSnapshot(
  value: unknown,
): NonNullable<ModelDescriptor["metadataSnapshot"]> {
  const metadata = asRecord(value, "model metadata");
  const cost = asRecord(metadata.cost, "model cost");
  const schemaVersion = requiredString(metadata.schema_version, "model metadata schema version");
  if (schemaVersion !== SAFE_MODEL_SNAPSHOT_SCHEMA) {
    throw new Error("unsupported model metadata schema version");
  }
  return {
    schemaVersion,
    apiId: requiredString(metadata.api_id, "model API id"),
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
      cacheRead: requiredNumber(cost.cache_read, "model cache read cost"),
      cacheWrite: requiredNumber(cost.cache_write, "model cache write cost"),
      tiers: optionalArray(cost.tiers, "model cost tiers").map((tier) => {
        const item = asRecord(tier, "model cost tier");
        return {
          input: requiredNumber(item.input, "tier input cost"),
          output: requiredNumber(item.output, "tier output cost"),
          cacheRead: requiredNumber(item.cache_read, "tier cache read cost"),
          cacheWrite: requiredNumber(item.cache_write, "tier cache write cost"),
          inputTokensAbove: requiredInteger(item.input_tokens_above, "tier input token threshold"),
        };
      }),
    },
    costConfigured: requiredBoolean(metadata.cost_configured, "model cost configured flag"),
    contextWindow: requiredUnsignedInteger(metadata.context_window, "model context window"),
    maxTokens: requiredUnsignedInteger(metadata.max_tokens, "model output limit"),
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

/** Reads an integer field matching Rust's signed integer wire type.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The safe integer field.
 * @throws When the value is not a safe integer.
 */
function requiredInteger(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new Error(`missing ${label}`);
  }
  return value;
}

/** Reads a nonnegative integer field matching Rust's unsigned integer type.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The nonnegative safe integer field.
 * @throws When the value is not a nonnegative safe integer.
 */
function requiredUnsignedInteger(value: unknown, label: string): number {
  const result = requiredInteger(value, label);
  if (result < 0) throw new Error(`missing ${label}`);
  return result;
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

/** Reads a Rust-defaulted optional array, rejecting malformed present values.
 * @param value Unknown field.
 * @param label Field description for diagnostics.
 * @returns The array contents, or an empty array when absent.
 * @throws When a present value is not an array.
 */
function optionalArray(value: unknown, label: string): unknown[] {
  if (value === undefined) return [];
  return requiredArray(value, label);
}

/** Returns one optional string property when present.
 * @param value Unknown field.
 * @param key Browser-facing property name.
 * @returns Empty or one optional string property.
 */
function optionalString(value: unknown, key: string): Record<string, string> {
  return typeof value === "string" ? { [key]: value } : {};
}
