/** @file Normalizes exact-base geospatial candidate responses. */
import type { GeoAttributeEditResponse, GeoGeometryEditResponse } from "@scriberig/protocol";

/** Normalizes one geometry edit candidate response.
 * @param value Raw Rust protocol response.
 * @returns Stable candidate and exact target identities.
 * @throws When any required identity is absent or malformed.
 */
export function normalizeGeoGeometryEditResponse(value: unknown): GeoGeometryEditResponse {
  const response = record(value, "geospatial geometry edit response");
  return {
    candidate: string(response.candidate, "candidate"),
    project: string(response.project, "project"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base snapshot"),
    layer: string(response.layer, "layer"),
    feature: string(response.feature, "feature"),
    path: string(response.path, "path"),
  };
}

/** Normalizes one attribute edit candidate response.
 * @param value Raw Rust protocol response.
 * @returns Stable candidate and exact target identities.
 * @throws When any required identity is absent or malformed.
 */
export function normalizeGeoAttributeEditResponse(value: unknown): GeoAttributeEditResponse {
  const response = record(value, "geospatial attribute edit response");
  return {
    candidate: string(response.candidate, "candidate"),
    project: string(response.project, "project"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base snapshot"),
    layer: string(response.layer, "layer"),
    feature: string(response.feature, "feature"),
    field: string(response.field, "field"),
    path: string(response.path, "path"),
  };
}

/** Reads one server spelling or public camel-case field.
 * @param value Object containing the field.
 * @param serverName Rust serde spelling.
 * @param publicName Public TypeScript spelling.
 * @returns The first defined spelling.
 */
function field(value: Record<string, unknown>, serverName: string, publicName: string): unknown {
  return value[serverName] ?? value[publicName];
}

/** Requires an object record.
 * @param value Value to inspect.
 * @param label Diagnostic label.
 * @returns Valid object record.
 * @throws When the value is not a record.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Requires a string value.
 * @param value Value to inspect.
 * @param label Diagnostic label.
 * @returns Valid string.
 * @throws When the value is not a string.
 */
function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new TypeError(`${label} must be a string`);
  return value;
}
