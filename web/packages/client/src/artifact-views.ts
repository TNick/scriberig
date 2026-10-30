/** @file Normalizes artifact wire responses for shared client consumers. */
import type {
  ArtifactInspectResponse,
  ArtifactMembersResponse,
  ArtifactEditCapability,
  ArtifactMemberSummary,
  ArtifactReference,
  ArtifactSummary,
} from "@scriberig/protocol";

/** Normalizes an optional exact artifact edit capability.
 * @param value Raw snake-case wire value, or absence from older responses.
 * @returns Validated capability or undefined when no catalog result exists.
 * @throws When a present capability is not a known protocol value.
 */
export function normalizeArtifactEditCapability(
  value: unknown,
): ArtifactEditCapability | undefined {
  if (value === undefined || value === null) return undefined;
  const capabilities: ArtifactEditCapability[] = [
    "unsupported",
    "inspect_only",
    "read_only_derived",
    "lossless_editable",
    "loss_aware_editable",
    "editable_with_opaque_preservation",
    "restricted_external_provider_edit",
  ];
  if (typeof value !== "string" || !capabilities.includes(value as ArtifactEditCapability)) {
    throw new TypeError("artifact edit capability is invalid");
  }
  return value as ArtifactEditCapability;
}

/** Normalizes a bounded physical/package-member page from the server wire model.
 * @param value Untyped response returned by the transport.
 * @returns Validated member page with camel-case fields.
 * @throws When required page or member fields are invalid.
 */
export function normalizeArtifactMembersResponse(value: unknown): ArtifactMembersResponse {
  const response = record(value, "artifact members response");
  if (!Array.isArray(response.members)) throw new TypeError("artifact members must be an array");
  const nextOffset = field(response, "next_offset", "nextOffset");
  return {
    project: string(response.project, "project"),
    artifact: string(response.artifact, "artifact"),
    members: response.members.map(normalizeMember),
    total: number(response.total, "total"),
    ...(nextOffset === undefined || nextOffset === null
      ? {}
      : { nextOffset: number(nextOffset, "next_offset") }),
  };
}

/** Normalizes snake-case server artifact metadata into the public client model.
 * @param value Untyped response returned by the transport.
 * @returns Validated artifact response with camel-case fields.
 * @throws When the response is missing required artifact contract fields.
 */
export function normalizeArtifactInspectResponse(value: unknown): ArtifactInspectResponse {
  const response = record(value, "artifact response");
  const artifacts = response.artifacts;
  if (!Array.isArray(artifacts))
    throw new TypeError("artifact response artifacts must be an array");
  const nextOffset = field(response, "next_offset", "nextOffset");
  return {
    project: string(response.project, "project"),
    artifacts: artifacts.map(normalizeArtifact),
    total: number(response.total, "total"),
    ...(nextOffset === undefined || nextOffset === null
      ? {}
      : { nextOffset: number(nextOffset, "next_offset") }),
  };
}

/** Normalizes one wire artifact summary.
 * @param value Untyped summary object.
 * @returns Validated semantic artifact summary.
 * @throws When a required field has an invalid type.
 */
function normalizeArtifact(value: unknown): ArtifactSummary {
  const item = record(value, "artifact summary");
  if (!Array.isArray(item.members) || !Array.isArray(item.references)) {
    throw new TypeError("artifact summary members and references must be arrays");
  }
  const adapterVersion = optionalString(
    field(item, "adapter_version", "adapterVersion"),
    "adapter_version",
  );
  const diagnostic = optionalString(item.diagnostic, "diagnostic");
  const observedDigest = optionalString(
    field(item, "observed_digest", "observedDigest"),
    "observed_digest",
  );
  const baselineDigest = optionalString(
    field(item, "baseline_digest", "baselineDigest"),
    "baseline_digest",
  );
  const rawSynchronizationStatus = field(item, "synchronization_status", "synchronizationStatus");
  const synchronizationStatus =
    rawSynchronizationStatus === undefined
      ? "unavailable"
      : string(rawSynchronizationStatus, "synchronization_status");
  if (
    !(["in-sync", "modified", "missing", "unavailable"] as const).includes(
      synchronizationStatus as "in-sync" | "modified" | "missing" | "unavailable",
    )
  ) {
    throw new TypeError("artifact synchronization status is invalid");
  }
  return {
    id: string(item.id, "id"),
    logicalKey: string(field(item, "logical_key", "logicalKey"), "logical_key"),
    kind: string(item.kind, "kind"),
    adapter: string(item.adapter, "adapter"),
    ...(adapterVersion === undefined ? {} : { adapterVersion }),
    scanId: string(field(item, "scan_id", "scanId"), "scan_id"),
    baselineDigest: baselineDigest ?? "",
    ...(observedDigest === undefined ? {} : { observedDigest }),
    synchronizationStatus: synchronizationStatus as ArtifactSummary["synchronizationStatus"],
    fidelity: string(item.fidelity, "fidelity"),
    opaquePreservation: string(
      field(item, "opaque_preservation", "opaquePreservation"),
      "opaque_preservation",
    ),
    ...(diagnostic === undefined ? {} : { diagnostic }),
    references: item.references.map(normalizeReference),
    members: item.members.map(normalizeMember),
    memberCount: number(field(item, "member_count", "memberCount"), "member_count"),
  };
}

/** Normalizes one typed external reference.
 * @param value Untyped reference object.
 * @returns Validated artifact reference.
 * @throws When a required reference field has an invalid type.
 */
function normalizeReference(value: unknown): ArtifactReference {
  const item = record(value, "artifact reference");
  return { kind: string(item.kind, "kind"), target: string(item.target, "target") };
}

/** Normalizes one artifact member descriptor.
 * @param value Untyped member object.
 * @returns Validated artifact member summary.
 * @throws When a required member field has an invalid type.
 */
function normalizeMember(value: unknown): ArtifactMemberSummary {
  const item = record(value, "artifact member");
  const projectPath = optionalString(field(item, "project_path", "projectPath"), "project_path");
  const containerPart = optionalString(
    field(item, "container_part", "containerPart"),
    "container_part",
  );
  return {
    key: string(item.key, "key"),
    ...(projectPath === undefined ? {} : { projectPath }),
    ...(containerPart === undefined ? {} : { containerPart }),
    digest: string(item.digest, "digest"),
  };
}

/** Requires a non-null object record.
 * @param value Candidate value.
 * @param label Diagnostic context.
 * @returns Object record.
 * @throws When the candidate is not a plain object.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Selects the server snake-case or client camel-case field spelling.
 * @param source Response object.
 * @param snake Server field name.
 * @param camel Public client field name.
 * @returns Present field value, if any.
 */
function field(source: Record<string, unknown>, snake: string, camel: string): unknown {
  return source[snake] ?? source[camel];
}

/** Requires a string field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated string.
 * @throws When the field is not a string.
 */
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new TypeError(`artifact ${name} must be a string`);
  return value;
}

/** Requires a numeric field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns Validated finite number.
 * @throws When the field is not a finite number.
 */
function number(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`artifact ${name} must be a finite number`);
  }
  return value;
}

/** Validates an optional string field.
 * @param value Candidate field value.
 * @param name Field name for diagnostics.
 * @returns The string or undefined when absent.
 * @throws When a present field is not a string.
 */
function optionalString(value: unknown, name: string): string | undefined {
  if (value === undefined || value === null) return undefined;
  return string(value, name);
}
