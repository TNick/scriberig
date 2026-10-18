/** @file Normalizes safe model library and session selection responses. */
import type {
  ModelLibraryResponse,
  ModelDescriptor,
  ModelProfile,
  SessionExecutionSelection,
  SessionExecutionChange,
} from "@scriberig/protocol";

/** Normalizes Rust or JavaScript model-library field casing.
 * @param value Unknown wire response.
 * @returns Safe model descriptors and profiles.
 * @throws When the response does not contain model and profile arrays.
 */
export function normalizeModelLibrary(value: unknown): ModelLibraryResponse {
  if (typeof value !== "object" || value === null) throw new Error("invalid model library");
  const record = value as Record<string, unknown>;
  return {
    models: Array.isArray(record.models) ? record.models.map(normalizeModel) : [],
    profiles: Array.isArray(record.profiles) ? record.profiles.map(normalizeProfile) : [],
  };
}

/** Normalizes one safe model descriptor.
 * @param value Unknown model descriptor.
 * @returns Public descriptor with browser field casing.
 * @throws When required identifiers are missing.
 */
function normalizeModel(value: unknown): ModelDescriptor {
  const record = asRecord(value, "model");
  const capabilities = asRecord(record.capabilities, "model capabilities");
  const levels = Array.isArray(record.thinking_levels)
    ? record.thinking_levels
    : Array.isArray(record.thinkingLevels)
      ? record.thinkingLevels
      : [];
  return {
    id: requiredString(record.id, "model id"),
    displayName: requiredString(record.display_name ?? record.displayName, "model name"),
    providerProfile: requiredString(
      record.provider_profile ?? record.providerProfile,
      "provider profile",
    ),
    providerModelId: requiredString(
      record.provider_model_id ?? record.providerModelId,
      "provider model id",
    ),
    capabilities: {
      streaming: capabilities.streaming === true,
      toolCalls: (capabilities.tool_calls ?? capabilities.toolCalls) === true,
      structuredOutput: (capabilities.structured_output ?? capabilities.structuredOutput) === true,
    },
    thinkingLevels: levels.map((level) => {
      const item = asRecord(level, "thinking level");
      return {
        id: requiredString(item.id, "thinking level id"),
        label: requiredString(item.label, "thinking level label"),
        ...optionalString(item.description, "description"),
        order: typeof item.order === "number" ? item.order : 0,
      };
    }),
    ...optionalString(
      record.default_thinking_level ?? record.defaultThinkingLevel,
      "defaultThinkingLevel",
    ),
  };
}

/** Normalizes one curated model profile without resolving credential values.
 * @param value Unknown model profile.
 * @returns Safe profile value.
 * @throws When required profile identifiers are missing.
 */
function normalizeProfile(value: unknown): ModelProfile {
  const record = asRecord(value, "model profile");
  const credential = record.credential;
  const credentialRecord =
    typeof credential === "object" && credential !== null
      ? (credential as Record<string, unknown>)
      : undefined;
  return {
    id: requiredString(record.id, "profile id"),
    displayName: requiredString(record.display_name ?? record.displayName, "profile name"),
    model: requiredString(record.model, "profile model"),
    connection: requiredString(record.connection, "profile connection"),
    ...(typeof credentialRecord?.id === "string"
      ? { credential: { id: credentialRecord.id } }
      : {}),
    ...optionalString(
      record.default_thinking_level ?? record.defaultThinkingLevel,
      "defaultThinkingLevel",
    ),
    enabled: record.enabled === true,
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
            ...optionalString(change.model, "model"),
            ...optionalString(change.thinking_level ?? change.thinkingLevel, "thinkingLevel"),
          },
        ];
      })
    : undefined;
  return {
    revision: typeof record.revision === "number" ? record.revision : 0,
    ...optionalString(record.model_profile ?? record.modelProfile, "modelProfile"),
    ...optionalString(record.model, "model"),
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

/** Returns one optional string property when present.
 * @param value Unknown field.
 * @param key Browser-facing property name.
 * @returns Empty or one optional string property.
 */
function optionalString(value: unknown, key: string): Record<string, string> {
  return typeof value === "string" ? { [key]: value } : {};
}
