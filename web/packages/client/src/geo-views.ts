/** @file Normalizes provider-neutral geospatial read responses. */
import type {
  GeoCoordinate,
  GeoCatalogResponse,
  GeoCrsView,
  GeoFeatureView,
  GeoFeatureQueryResponse,
  GeoFeaturesResponse,
  GeoFieldView,
  GeoGeometry,
  GeoLayerView,
  GeoValue,
} from "@scriberig/protocol";

/** Normalizes a bounded Rust GeoPackage catalog response.
 * @param value Raw transport response.
 * @returns Snapshot-bound dataset and ordered layer summaries.
 * @throws When catalog metadata or a layer schema is malformed.
 */
export function normalizeGeoCatalogResponse(value: unknown): GeoCatalogResponse {
  const response = record(value, "GeoPackage catalog");
  if (!Array.isArray(response.layers)) throw new TypeError("layers must be an array");
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    dataset: string(response.dataset, "dataset"),
    sourceDigest: string(response.source_digest, "source digest"),
    layers: response.layers.map(normalizeLayer),
    total: number(response.total, "total"),
    hasMore: boolean(response.has_more, "has_more"),
  };
}

/** Normalizes one semantic GeoPackage layer summary.
 * @param value Raw layer response.
 * @returns Typed schema, CRS, extent, and capability summary.
 * @throws When required schema or capability fields are malformed.
 */
function normalizeLayer(value: unknown): GeoLayerView {
  const layer = record(value, "layer");
  if (!Array.isArray(layer.fields) || !Array.isArray(layer.diagnostics)) {
    throw new TypeError("layer fields and diagnostics must be arrays");
  }
  if (typeof layer.editable !== "boolean") throw new TypeError("editable must be a boolean");
  const geometryType = layer.geometry_type;
  if (
    geometryType != null &&
    geometryType !== "point" &&
    geometryType !== "line_string" &&
    geometryType !== "polygon" &&
    geometryType !== "geometry_collection"
  ) {
    throw new TypeError("geometry type is unsupported");
  }
  const extent = layer.extent == null ? undefined : normalizeExtent(layer.extent);
  const crs = layer.crs == null ? undefined : normalizeCrs(layer.crs);
  return {
    id: string(layer.id, "layer id"),
    dataset: string(layer.dataset, "dataset id"),
    name: string(layer.name, "layer name"),
    fields: layer.fields.map(normalizeField),
    ...(crs === undefined ? {} : { crs }),
    ...(geometryType == null ? {} : { geometryType }),
    featureCount: number(layer.feature_count, "feature count"),
    ...(extent === undefined ? {} : { extent }),
    editable: layer.editable,
    diagnostics: layer.diagnostics.map((item) => string(item, "layer diagnostic")),
  };
}

/** Normalizes one provider-neutral layer field.
 * @param value Raw field response.
 * @returns Typed field schema.
 * @throws When field names, types, or nullability are invalid.
 */
function normalizeField(value: unknown): GeoFieldView {
  const field = record(value, "field");
  const fieldType = field.field_type;
  if (
    fieldType !== "text" &&
    fieldType !== "number" &&
    fieldType !== "boolean" &&
    fieldType !== "date_time" &&
    fieldType !== "binary"
  ) {
    throw new TypeError("field type is unsupported");
  }
  return {
    name: string(field.name, "field name"),
    fieldType,
    nullable: boolean(field.nullable, "field nullable"),
  };
}

/** Normalizes explicit CRS metadata.
 * @param value Raw CRS record.
 * @returns Typed optional authority/code/definition values.
 * @throws When a provided CRS member is not text.
 */
function normalizeCrs(value: unknown): GeoCrsView {
  const crs = record(value, "CRS");
  return {
    ...(crs.authority == null ? {} : { authority: string(crs.authority, "CRS authority") }),
    ...(crs.code == null ? {} : { code: string(crs.code, "CRS code") }),
    ...(crs.definition == null ? {} : { definition: string(crs.definition, "CRS definition") }),
  };
}

/** Normalizes an exact four-ordinate layer extent.
 * @param value Raw extent array.
 * @returns Minimum x/y and maximum x/y decimal text.
 * @throws When the extent does not contain exactly four strings.
 */
function normalizeExtent(value: unknown): [string, string, string, string] {
  if (!Array.isArray(value) || value.length !== 4) {
    throw new TypeError("extent must contain four coordinates");
  }
  return [
    string(value[0], "extent minimum x"),
    string(value[1], "extent minimum y"),
    string(value[2], "extent maximum x"),
    string(value[3], "extent maximum y"),
  ];
}

/** Normalizes an HTTP geospatial feature page into the public typed view.
 * @param value Raw Rust protocol response.
 * @returns Typed bounded feature page with exact semantic context.
 * @throws When required metadata, geometry, or attribute values are malformed.
 */
export function normalizeGeoFeaturesResponse(value: unknown): GeoFeaturesResponse {
  const response = record(value, "geospatial feature page");
  if (!Array.isArray(response.features)) throw new TypeError("features must be an array");
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    layer: string(response.layer, "layer"),
    features: response.features.map(normalizeFeature),
    hasMore: boolean(response.has_more, "has_more"),
  };
}

/** Normalizes a bounded spatial query with explicit source provenance.
 * @param value Raw Rust protocol response.
 * @returns Typed query page and exact source evaluation metadata.
 * @throws When query rows or provenance fields are malformed.
 */
export function normalizeGeoFeatureQueryResponse(value: unknown): GeoFeatureQueryResponse {
  const response = record(value, "geospatial query response");
  if (!Array.isArray(response.features) || !Array.isArray(response.diagnostics)) {
    throw new TypeError("geospatial query features and diagnostics must be arrays");
  }
  const layerCrs = response.layer_crs ?? response.layerCrs;
  const coordinateTransform = response.coordinate_transform ?? response.coordinateTransform;
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    layer: string(response.layer, "layer"),
    features: response.features.map(normalizeFeature),
    hasMore: boolean(field(response, "has_more", "hasMore"), "has_more"),
    sourceRevision: string(field(response, "source_revision", "sourceRevision"), "source revision"),
    sourceDigest: string(field(response, "source_digest", "sourceDigest"), "source digest"),
    provider: string(response.provider, "provider"),
    ...(layerCrs == null ? {} : { layerCrs: normalizeCrs(layerCrs) }),
    strategy: string(response.strategy, "strategy"),
    ...(coordinateTransform == null
      ? {}
      : { coordinateTransform: string(coordinateTransform, "coordinate transform") }),
    diagnostics: response.diagnostics.map((item) => string(item, "query diagnostic")),
  };
}

/** Normalizes one semantic feature record.
 * @param value Raw feature response.
 * @returns Typed feature view.
 * @throws When feature identities or attributes are malformed.
 */
function normalizeFeature(value: unknown): GeoFeatureView {
  const feature = record(value, "feature");
  const rawAttributes = record(feature.attributes, "feature attributes");
  return {
    id: string(feature.id, "feature id"),
    ...(feature.native_id == null ? {} : { nativeId: string(feature.native_id, "native id") }),
    ...(feature.geometry == null ? {} : { geometry: normalizeGeometry(feature.geometry) }),
    attributes: Object.fromEntries(
      Object.entries(rawAttributes).map(([key, item]) => [key, normalizeValue(item)]),
    ),
  };
}

/** Normalizes one recursively typed geometry record.
 * @param value Raw protocol geometry.
 * @returns Typed geometry with exact decimal coordinates.
 * @throws When its tag or coordinate structure is malformed.
 */
function normalizeGeometry(value: unknown): GeoGeometry {
  const geometry = record(value, "geometry");
  switch (geometry.kind) {
    case "point":
      return { kind: "point", coordinates: normalizeCoordinate(geometry.coordinates) };
    case "line_string":
      return {
        kind: "line_string",
        coordinates: array(geometry.coordinates, "line coordinates").map(normalizeCoordinate),
      };
    case "polygon":
      return {
        kind: "polygon",
        coordinates: array(geometry.coordinates, "polygon rings").map((ring) =>
          array(ring, "polygon ring").map(normalizeCoordinate),
        ),
      };
    case "collection":
      return {
        kind: "collection",
        coordinates: array(geometry.coordinates, "collection geometries").map(normalizeGeometry),
      };
    default:
      throw new TypeError("geometry kind is unsupported");
  }
}

/** Normalizes one coordinate without converting exact decimal text to numbers.
 * @param value Raw coordinate record.
 * @returns Exact coordinate ordinates.
 * @throws When required coordinate text is missing or invalid.
 */
function normalizeCoordinate(value: unknown): GeoCoordinate {
  const coordinate = record(value, "coordinate");
  return {
    x: string(coordinate.x, "x coordinate"),
    y: string(coordinate.y, "y coordinate"),
    ...(coordinate.z == null ? {} : { z: string(coordinate.z, "z coordinate") }),
    ...(coordinate.measure == null
      ? {}
      : { measure: string(coordinate.measure, "measure coordinate") }),
  };
}

/** Normalizes one tagged attribute value.
 * @param value Raw typed attribute.
 * @returns Typed provider-neutral value.
 * @throws When its tag or scalar value is invalid.
 */
function normalizeValue(value: unknown): GeoValue {
  const item = record(value, "attribute value");
  switch (item.kind) {
    case "null":
      return { kind: "null" };
    case "text":
    case "number":
    case "date_time":
      return { kind: item.kind, value: string(item.value, "attribute text") };
    case "boolean":
      return { kind: "boolean", value: boolean(item.value, "attribute boolean") };
    default:
      throw new TypeError("attribute value kind is unsupported");
  }
}

/** Requires an object value with string keys.
 * @param value Value to inspect.
 * @param name Diagnostic label for the expected record.
 * @returns The validated object record.
 * @throws When the value is null, an array, or a scalar.
 */
function record(value: unknown, name: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${name} must be an object`);
  }
  return value as Record<string, unknown>;
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

/** Requires an array value.
 * @param value Value to inspect.
 * @param name Diagnostic label for the expected array.
 * @returns The validated array.
 * @throws When the value is not an array.
 */
function array(value: unknown, name: string): unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`${name} must be an array`);
  return value;
}

/** Requires a string value.
 * @param value Value to inspect.
 * @param name Diagnostic label for the expected string.
 * @returns The validated string.
 * @throws When the value is not a string.
 */
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new TypeError(`${name} must be a string`);
  return value;
}

/** Requires a boolean value.
 * @param value Value to inspect.
 * @param name Diagnostic label for the expected boolean.
 * @returns The validated boolean.
 * @throws When the value is not a boolean.
 */
function boolean(value: unknown, name: string): boolean {
  if (typeof value !== "boolean") throw new TypeError(`${name} must be a boolean`);
  return value;
}

/** Requires a safe non-negative integer value.
 * @param value Value to inspect.
 * @param name Diagnostic label for the expected integer.
 * @returns The validated integer.
 * @throws When the value is not a non-negative safe integer.
 */
function number(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${name} must be a non-negative safe integer`);
  }
  return value;
}
