/** @file Normalizes projection style responses from the Rust protocol. */
import type {
  ProjectionStyleFidelityReport,
  ProjectionStylePurpose,
  ProjectionStyleReference,
  ProjectionStyleResolveResponse,
  ProjectionStyleSchemaResponse,
  ProjectionStyleSettingResponse,
} from "@scriberig/protocol";

/** Normalizes an adapter schema returned by the Rust protocol.
 * @param value Raw protocol response.
 * @returns Typed schema with camel-case package fields.
 * @throws When a schema property or enum tag is malformed.
 */
export function normalizeProjectionStyleSchema(
  value: unknown,
): ProjectionStyleSchemaResponse | undefined {
  if (value === null || value === undefined) return undefined;
  const schema = record(value, "projection style schema");
  const reference = record(schema.reference, "schema reference");
  if (!Array.isArray(schema.projection_kinds) || !Array.isArray(schema.properties)) {
    throw new TypeError("projection style schema collections must be arrays");
  }
  return {
    reference: {
      adapterId: string(reference.adapter_id, "adapter id"),
      version: string(reference.version, "schema version"),
    },
    projectionKinds: schema.projection_kinds.map((item) => string(item, "projection kind")),
    properties: schema.properties.map((item) => {
      const property = record(item, "style property");
      if (!Array.isArray(property.allowed_values))
        throw new TypeError("allowed values must be an array");
      const description = optionalString(property.description);
      const minimum = optionalNumber(property.minimum);
      const maximum = optionalNumber(property.maximum);
      return {
        key: string(property.key, "property key"),
        label: string(property.label, "property label"),
        ...(description === undefined ? {} : { description }),
        kind: enumValue(
          property.kind,
          {
            Boolean: "boolean",
            Integer: "integer",
            Enum: "enum",
            Text: "text",
            Opaque: "opaque",
          },
          "style property kind",
        ),
        allowedValues: property.allowed_values.map((entry) => string(entry, "allowed value")),
        ...(minimum === undefined ? {} : { minimum }),
        ...(maximum === undefined ? {} : { maximum }),
        default: byteArray(property.default, "default value"),
        applicability: enumValue(
          property.applicability,
          {
            ViewOnly: "viewOnly",
            MaterializationOnly: "materializationOnly",
            Both: "both",
          },
          "style applicability",
        ),
        affectsFidelity: boolean(property.affects_fidelity, "affects fidelity"),
        requiredForMaterialization: boolean(
          property.required_for_materialization,
          "required for materialization",
        ),
      };
    }),
  };
}

/** Normalizes an effective style and its exact source/fidelity evidence.
 * @param value Raw protocol response.
 * @returns Typed style resolution.
 * @throws When required style bytes, provenance, or fidelity fields are malformed.
 */
export function normalizeProjectionStyleResolution(value: unknown): ProjectionStyleResolveResponse {
  const result = record(value, "projection style resolution");
  if (!Array.isArray(result.applied)) throw new TypeError("applied style sources must be an array");
  const schema = result.schema == null ? undefined : normalizeSchemaReference(result.schema);
  const report = record(result.fidelity, "style fidelity");
  if (!Array.isArray(report.omitted_categories))
    throw new TypeError("omitted categories must be an array");
  const explanation = optionalString(report.explanation);
  const fidelity: ProjectionStyleFidelityReport = {
    fidelity: enumValue(
      report.fidelity,
      {
        LosslessEditable: "losslessEditable",
        LossAwareEditable: "lossAwareEditable",
        ReadOnlyLossy: "readOnlyLossy",
      },
      "style fidelity",
    ),
    omittedCategories: report.omitted_categories.map((entry) => string(entry, "omitted category")),
    ...(explanation === undefined ? {} : { explanation }),
  };
  return {
    purpose: normalizePurpose(result.purpose),
    reference: normalizeStyleReference(result.reference),
    value: byteArray(result.value, "resolved style"),
    ...(schema === undefined ? {} : { schema }),
    applied: result.applied.map((item) => {
      const source = record(item, "style source");
      const revision = optionalString(source.revision);
      const generation = optionalNumber(source.generation);
      return {
        kind: string(source.kind, "source kind"),
        identity: string(source.identity, "source identity"),
        ...(revision === undefined ? {} : { revision }),
        ...(generation === undefined ? {} : { generation }),
        value: byteArray(source.value, "source style"),
      };
    }),
    fidelity,
  };
}

/** Normalizes the authoritative result of a style setting write.
 * @param value Raw protocol response.
 * @returns Stored bytes and revision when present.
 * @throws When a style byte array or revision is malformed.
 */
export function normalizeProjectionStyleSetting(value: unknown): ProjectionStyleSettingResponse {
  const result = record(value, "projection style setting");
  const style = result.style == null ? undefined : byteArray(result.style, "stored style");
  const revision = result.revision == null ? undefined : number(result.revision, "revision");
  return {
    ...(style === undefined ? {} : { style }),
    ...(revision === undefined ? {} : { revision }),
  };
}

/** Normalizes a serialized style schema reference.
 * @param value Raw protocol schema reference.
 * @returns Typed schema identity.
 * @throws When the schema identity is malformed.
 */
function normalizeSchemaReference(value: unknown): { adapterId: string; version: string } {
  const reference = record(value, "style schema reference");
  return {
    adapterId: string(reference.adapter_id, "adapter id"),
    version: string(reference.version, "schema version"),
  };
}

/** Normalizes Rust's externally tagged style reference enum.
 * @param value Raw protocol style reference.
 * @returns Typed inline or immutable revision reference.
 * @throws When the enum variant or reference identity is invalid.
 */
function normalizeStyleReference(value: unknown): ProjectionStyleReference {
  if (value === "Inline") return { kind: "inline" };
  const reference = record(value, "style reference");
  const revision = record(reference.Revision, "revision style reference");
  return {
    kind: "revision",
    entity: string(revision.entity, "style entity"),
    revision: string(revision.revision, "style revision"),
  };
}

/** Normalizes a Rust projection purpose enum.
 * @param value Raw protocol purpose.
 * @returns Typed purpose.
 * @throws When the purpose variant is unknown.
 */
function normalizePurpose(value: unknown): ProjectionStylePurpose {
  return enumValue(
    value,
    {
      InteractiveView: "interactiveView",
      SharedMaterialization: "sharedMaterialization",
      ExplicitExport: "explicitExport",
    },
    "style purpose",
  );
}

/** Narrows an unknown value to a protocol record.
 * @param value Value to inspect.
 * @param label Field name for diagnostics.
 * @returns Object record.
 * @throws When value is not a non-null object.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Validates a required string field.
 * @param value Value to inspect.
 * @param label Field name for diagnostics.
 * @returns The required string.
 * @throws When value is not a string.
 */
function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new TypeError(`${label} must be a string`);
  return value;
}

/** Validates one optional string field.
 * @param value Value to inspect.
 * @returns The optional string.
 * @throws When a present value is not a string.
 */
function optionalString(value: unknown): string | undefined {
  return value == null ? undefined : string(value, "optional string");
}

/** Validates one required boolean field.
 * @param value Value to inspect.
 * @param label Field name for diagnostics.
 * @returns The boolean value.
 * @throws When value is not a boolean.
 */
function boolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw new TypeError(`${label} must be a boolean`);
  return value;
}

/** Validates one required number field.
 * @param value Value to inspect.
 * @param label Field name for diagnostics.
 * @returns The finite numeric value.
 * @throws When value is not a finite number.
 */
function number(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value))
    throw new TypeError(`${label} must be numeric`);
  return value;
}

/** Validates one optional number field.
 * @param value Value to inspect.
 * @returns The finite numeric value when present.
 * @throws When a present value is not finite.
 */
function optionalNumber(value: unknown): number | undefined {
  return value == null ? undefined : number(value, "optional number");
}

/** Validates a protocol byte array.
 * @param value Value to inspect.
 * @param label Field name for diagnostics.
 * @returns A copied array of byte values.
 * @throws When the value contains a non-integer outside the byte range.
 */
function byteArray(value: unknown, label: string): number[] {
  if (
    !Array.isArray(value) ||
    value.some((item) => !Number.isInteger(item) || item < 0 || item > 255)
  ) {
    throw new TypeError(`${label} must be an array of bytes`);
  }
  return [...value] as number[];
}

/** Maps one serialized Rust enum variant to its TypeScript literal.
 * @param value Raw enum variant.
 * @param mapping Accepted Rust to TypeScript labels.
 * @param label Enum name for diagnostics.
 * @returns The mapped TypeScript literal.
 * @throws When the Rust enum variant is not recognized.
 */
function enumValue<T extends string>(value: unknown, mapping: Record<string, T>, label: string): T {
  if (typeof value !== "string" || mapping[value] === undefined) {
    throw new TypeError(`${label} has an unknown variant`);
  }
  return mapping[value]!;
}
