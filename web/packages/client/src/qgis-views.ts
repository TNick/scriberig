/** @file Normalizes QGIS project references and their resolved semantic links. */
import type {
  QgisLayerGroupView,
  QgisLayerOrderResponse,
  QgisLayerRenameResponse,
  QgisLayerReferenceView,
  QgisProjectResponse,
} from "@scriberig/protocol";

/** Normalizes a QGIS layer-order candidate response.
 * @param value Raw Rust protocol response.
 * @returns Typed candidate handle for shared review and apply.
 * @throws When a required project, snapshot, candidate, or path is missing.
 */
export function normalizeQgisLayerOrderResponse(value: unknown): QgisLayerOrderResponse {
  const response = record(value, "QGIS layer order response");
  return {
    candidate: string(field(response, "candidate_id", "candidate"), "candidate"),
    project: string(response.project, "project"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base snapshot"),
    path: string(response.path, "path"),
  };
}

/** Normalizes a QGIS layer rename candidate response.
 * @param value Raw Rust protocol response.
 * @returns Typed candidate handle for shared review and apply.
 * @throws When a required project, snapshot, candidate, or reference ID is missing.
 */
export function normalizeQgisLayerRenameResponse(value: unknown): QgisLayerRenameResponse {
  const response = record(value, "QGIS layer rename response");
  return {
    candidate: string(field(response, "candidate_id", "candidate"), "candidate"),
    project: string(response.project, "project"),
    baseSnapshot: string(field(response, "base_snapshot", "baseSnapshot"), "base snapshot"),
    reference: string(response.reference, "layer reference"),
    path: string(response.path, "path"),
  };
}

/** Normalizes a provider-neutral QGIS project response.
 * @param value Raw Rust protocol response.
 * @returns Exact-snapshot project view and typed reference links.
 * @throws When project, group, or layer identities are malformed.
 */
export function normalizeQgisProjectResponse(value: unknown): QgisProjectResponse {
  const response = record(value, "QGIS project response");
  if (!Array.isArray(response.layers) || !Array.isArray(response.groups)) {
    throw new TypeError("QGIS project layers and groups must be arrays");
  }
  const crs = response.crs == null ? undefined : normalizeCrs(response.crs);
  return {
    project: string(response.project, "project"),
    path: string(response.path, "path"),
    snapshot: string(response.snapshot, "snapshot"),
    revision: string(response.revision, "revision"),
    id: string(response.id, "QGIS project id"),
    ...(response.title == null ? {} : { title: string(response.title, "title") }),
    ...(crs === undefined ? {} : { crs }),
    layers: response.layers.map(normalizeLayerReference),
    groups: response.groups.map(normalizeGroup),
  };
}

/** Normalizes one QGIS reference while preserving its separate resolved GeoLayer ID.
 * @param value Raw QGIS reference.
 * @returns Typed authored reference and optional resolved links.
 * @throws When required identifiers or display values are malformed.
 */
function normalizeLayerReference(value: unknown): QgisLayerReferenceView {
  const layer = record(value, "QGIS layer reference");
  return {
    id: string(layer.id, "layer reference id"),
    name: string(layer.name, "layer name"),
    dataSource: string(field(layer, "data_source", "dataSource"), "data source"),
    ...optionalString(layer.path, "path"),
    ...optionalString(field(layer, "resolved_path", "resolvedPath"), "resolved path"),
    ...optionalString(field(layer, "source_layer", "sourceLayer"), "source layer"),
    ...optionalString(layer.dataset, "dataset"),
    ...optionalString(layer.layer, "resolved GeoLayer"),
    ...optionalString(
      field(layer, "resolution_diagnostic", "resolutionDiagnostic"),
      "resolution diagnostic",
    ),
    visible: boolean(layer.visible, "visibility"),
    ...optionalString(layer.renderer, "renderer"),
    ...optionalString(field(layer, "scale_range", "scaleRange"), "scale range"),
  };
}

/** Normalizes one project layer group.
 * @param value Raw group response.
 * @returns Typed group identity and ordered QGIS references.
 * @throws When group names or ordered references are invalid.
 */
function normalizeGroup(value: unknown): QgisLayerGroupView {
  const group = record(value, "QGIS layer group");
  if (!Array.isArray(group.layers)) throw new TypeError("QGIS group layers must be an array");
  return {
    id: string(group.id, "group id"),
    name: string(group.name, "group name"),
    layers: group.layers.map((layer) => string(layer, "group layer reference")),
  };
}

/** Normalizes optional CRS identity fields.
 * @param value Raw CRS response.
 * @returns Typed authority, code, and definition.
 * @throws When a present CRS field is not text.
 */
function normalizeCrs(value: unknown): NonNullable<QgisProjectResponse["crs"]> {
  const crs = record(value, "QGIS project CRS");
  return {
    ...optionalString(crs.authority, "CRS authority"),
    ...optionalString(crs.code, "CRS code"),
    ...optionalString(crs.definition, "CRS definition"),
  };
}

/** Normalizes a safe optional text field.
 * @param value Raw optional field value.
 * @param name Diagnostic label.
 * @returns An optional property record.
 * @throws When a present value is not text.
 */
function optionalString(value: unknown, name: string): Record<string, string> {
  return value == null
    ? {}
    : { [name === "resolved GeoLayer" ? "layer" : camelName(name)]: string(value, name) };
}

/** Maps one readable label to its public property name.
 * @param name Diagnostic field label.
 * @returns Stable camel-case property name.
 */
function camelName(name: string): string {
  const names: Record<string, string> = {
    path: "path",
    "resolved path": "resolvedPath",
    "source layer": "sourceLayer",
    dataset: "dataset",
    "resolution diagnostic": "resolutionDiagnostic",
    renderer: "renderer",
    "scale range": "scaleRange",
    "CRS authority": "authority",
    "CRS code": "code",
    "CRS definition": "definition",
  };
  return names[name] ?? name;
}

/** Reads one Rust or public TypeScript field spelling.
 * @param value Object containing the field.
 * @param serverName Rust serde spelling.
 * @param publicName TypeScript spelling.
 * @returns The first non-null field value.
 */
function field(value: Record<string, unknown>, serverName: string, publicName: string): unknown {
  return value[serverName] ?? value[publicName];
}

/** Requires a non-array object record.
 * @param value Value to inspect.
 * @param label Diagnostic name.
 * @returns Object record.
 * @throws When the value is not an object record.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Requires a string field.
 * @param value Value to inspect.
 * @param label Diagnostic name.
 * @returns String value.
 * @throws When the value is not a string.
 */
function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new TypeError(`${label} must be a string`);
  return value;
}

/** Requires a boolean field.
 * @param value Value to inspect.
 * @param label Diagnostic name.
 * @returns Boolean value.
 * @throws When the value is not a boolean.
 */
function boolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw new TypeError(`${label} must be a boolean`);
  return value;
}
