/** @file Coordinates exact-snapshot geospatial catalog, feature, and edit operations. */
import type {
  GeoAttributeEditRequest,
  GeoAttributeEditResponse,
  GeoCatalogResponse,
  GeoFeatureBounds,
  GeoFeatureQueryResponse,
  GeoFeatureView,
  GeoFeaturesResponse,
  GeoGeometry,
  GeoGeometryEditResponse,
  GeoLayerView,
  GeoValue,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Controls bounded GeoPackage reads and exact-base feature candidate edits.
 *
 * Public members: `readCatalog`, `readFeatures`, `queryEnvelope`,
 * `proposeAttribute`, `proposeGeometry`, `currentCatalog`, and `currentFeatures`.
 * Private members: client, project, source path, exact snapshot, and latest page state.
 */
export class GeoPageController {
  private catalog?: GeoCatalogResponse;
  private pages = new Map<string, GeoFeaturesResponse | GeoFeatureQueryResponse>();

  /** Creates a controller bound to one exact project source and snapshot.
   * @param client Shared GeoPackage and feature operations.
   * @param project Stable owning project identity.
   * @param path Project-relative GeoPackage path.
   * @param snapshot Exact immutable snapshot identity.
   * @param pageSize Maximum features returned per request, from one through one thousand.
   * @throws When page size is outside the shared operation limit.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      | "inspectGeoPackage"
      | "geoFeatures"
      | "queryGeoFeatures"
      | "proposeGeoAttributeEdit"
      | "proposeGeoGeometryEdit"
    >,
    private readonly project: string,
    private readonly path: string,
    private readonly snapshot: string,
    private readonly pageSize = 100,
  ) {
    if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 1000) {
      throw new RangeError("geospatial page size must be between one and one thousand");
    }
    if (path.length === 0 || path.startsWith("/") || path.split("/").includes("..")) {
      throw new TypeError("GeoPackage path must be a project-relative path");
    }
  }

  /** Reads one bounded semantic layer catalog page.
   * @param offset Zero-based layer offset.
   * @param limit Maximum layer count, from one through two hundred.
   * @param signal Optional request cancellation signal.
   * @returns Exact-snapshot layer metadata and source digest.
   * @throws When response identity or page bounds do not match this controller.
   */
  async readCatalog(offset = 0, limit = 100, signal?: AbortSignal): Promise<GeoCatalogResponse> {
    if (
      !Number.isInteger(offset) ||
      offset < 0 ||
      !Number.isInteger(limit) ||
      limit < 1 ||
      limit > 200
    ) {
      throw new RangeError("GeoPackage catalog page bounds are invalid");
    }
    const catalog = await this.client.inspectGeoPackage(
      { project: this.project, path: this.path, snapshot: this.snapshot, offset, limit },
      signal,
    );
    if (
      catalog.project !== this.project ||
      catalog.snapshot !== this.snapshot ||
      catalog.layers.some((layer) => layer.dataset !== catalog.dataset)
    ) {
      throw new TypeError("GeoPackage catalog identity does not match the requested snapshot");
    }
    this.catalog = catalog;
    return catalog;
  }

  /** Reads one bounded page of features from a known exact-snapshot layer.
   * @param layer Stable layer identity present in the loaded catalog.
   * @param offset Zero-based source feature offset.
   * @param signal Optional request cancellation signal.
   * @returns Exact-snapshot semantic feature page.
   * @throws When the layer is unknown or the response identity mismatches.
   */
  async readFeatures(
    layer: string,
    offset = 0,
    signal?: AbortSignal,
  ): Promise<GeoFeaturesResponse> {
    this.requireLayer(layer);
    if (!Number.isInteger(offset) || offset < 0) throw new RangeError("feature offset is invalid");
    const page = await this.client.geoFeatures(
      {
        project: this.project,
        path: this.path,
        snapshot: this.snapshot,
        layer,
        offset,
        limit: this.pageSize,
      },
      signal,
    );
    this.verifyFeatures(page, layer);
    this.pages.set(layer, page);
    return page;
  }

  /** Reads one page matching a CRS-explicit spatial window.
   * @param layer Stable layer identity present in the loaded catalog.
   * @param bounds Ordered decimal bounds and their coordinate reference system.
   * @param offset Zero-based matching feature offset.
   * @param signal Optional request cancellation signal.
   * @returns Bounded matching features with source evaluation provenance.
   * @throws When bounds, CRS, layer, or response identity is invalid.
   */
  async queryEnvelope(
    layer: string,
    bounds: GeoFeatureBounds,
    offset = 0,
    signal?: AbortSignal,
  ): Promise<GeoFeatureQueryResponse> {
    this.requireLayer(layer);
    validateBounds(bounds);
    if (!Number.isInteger(offset) || offset < 0) throw new RangeError("query offset is invalid");
    const result = await this.client.queryGeoFeatures(
      {
        project: this.project,
        path: this.path,
        snapshot: this.snapshot,
        layer,
        predicate: { kind: "envelope_overlaps", bounds },
        offset,
        limit: this.pageSize,
      },
      signal,
    );
    this.verifyFeatures(result, layer);
    this.pages.set(layer, result);
    return result;
  }

  /** Creates a typed attribute candidate for a feature on the loaded page.
   * @param layer Stable layer identity.
   * @param feature Stable feature identity on the current page.
   * @param field Authored field name from the semantic schema.
   * @param value Typed replacement value.
   * @param signal Optional request cancellation signal.
   * @returns Shared candidate handle.
   * @throws When fidelity, target membership, or field type does not permit editing.
   */
  async proposeAttribute(
    layer: string,
    feature: string,
    field: string,
    value: GeoValue,
    signal?: AbortSignal,
  ): Promise<GeoAttributeEditResponse> {
    const [targetLayer] = this.requireEditableFeature(layer, feature);
    const schema = targetLayer.fields.find((candidate) => candidate.name === field);
    if (schema === undefined || schema.fieldType === "binary") {
      throw new TypeError("attribute field is unavailable for semantic editing");
    }
    if (value.kind === "null" ? !schema.nullable : !valueMatchesField(value, schema.fieldType)) {
      throw new TypeError("typed value does not match the selected geospatial field");
    }
    const result = await this.client.proposeGeoAttributeEdit(
      {
        project: this.project,
        path: this.path,
        snapshot: this.snapshot,
        layer,
        feature,
        field,
        value,
      } satisfies GeoAttributeEditRequest,
      signal,
    );
    verifyCandidate(result, this.project, this.snapshot, layer, feature, this.path);
    if (result.field !== field) throw new TypeError("candidate response field identity mismatched");
    return result;
  }

  /** Creates a typed geometry candidate for a feature on the loaded page.
   * @param layer Stable layer identity.
   * @param feature Stable feature identity on the current page.
   * @param geometry Replacement geometry in the layer's declared CRS.
   * @param signal Optional request cancellation signal.
   * @returns Shared candidate handle.
   * @throws When fidelity, target membership, or geometry type disallows editing.
   */
  async proposeGeometry(
    layer: string,
    feature: string,
    geometry: GeoGeometry,
    signal?: AbortSignal,
  ): Promise<GeoGeometryEditResponse> {
    const [targetLayer] = this.requireEditableFeature(layer, feature);
    if (targetLayer.geometryType !== undefined && geometry.kind !== targetLayer.geometryType) {
      throw new TypeError("replacement geometry does not match the layer geometry type");
    }
    const result = await this.client.proposeGeoGeometryEdit(
      {
        project: this.project,
        path: this.path,
        snapshot: this.snapshot,
        layer,
        feature,
        geometry,
      },
      signal,
    );
    verifyCandidate(result, this.project, this.snapshot, layer, feature, this.path);
    return result;
  }

  /** Returns the latest catalog page.
   * @returns Catalog page, or undefined before a successful read.
   */
  currentCatalog(): GeoCatalogResponse | undefined {
    return this.catalog;
  }

  /** Returns the most recent feature page or spatial query for one layer.
   * @param layer Stable layer identity.
   * @returns Loaded bounded page, or undefined before a successful read.
   */
  currentFeatures(layer: string): GeoFeaturesResponse | GeoFeatureQueryResponse | undefined {
    return this.pages.get(layer);
  }

  /** Resolves one layer in the current catalog.
   * @param layer Stable semantic layer identity.
   * @returns Exact layer summary.
   * @throws When no matching layer is loaded.
   */
  private requireLayer(layer: string): GeoLayerView {
    const result = this.catalog?.layers.find((candidate) => candidate.id === layer);
    if (result === undefined) throw new RangeError("geospatial layer is not in the loaded catalog");
    return result;
  }

  /** Requires a feature target and layer with complete editable fidelity.
   * @param layer Stable layer identity.
   * @param feature Stable semantic feature identity.
   * @returns Exact layer and feature currently loaded.
   * @throws When layer, feature, or edit capability is unavailable.
   */
  private requireEditableFeature(layer: string, feature: string): [GeoLayerView, GeoFeatureView] {
    const summary = this.requireLayer(layer);
    if (!summary.editable) {
      throw new TypeError(summary.diagnostics[0] ?? "This geospatial layer is read-only.");
    }
    const target = this.pages.get(layer)?.features.find((candidate) => candidate.id === feature);
    if (target === undefined)
      throw new RangeError("feature is not present on the loaded layer page");
    return [summary, target];
  }

  /** Verifies response project, snapshot, and layer identities.
   * @param response Loaded feature or query page.
   * @param layer Expected stable layer identity.
   * @throws When response scope does not match the controller.
   */
  private verifyFeatures(
    response: GeoFeaturesResponse | GeoFeatureQueryResponse,
    layer: string,
  ): void {
    if (
      response.project !== this.project ||
      response.snapshot !== this.snapshot ||
      response.layer !== layer
    ) {
      throw new TypeError("feature page identity does not match the requested snapshot");
    }
  }
}

/** Checks exact-base candidate response identity.
 * @param response Candidate operation response.
 * @param project Expected project identity.
 * @param snapshot Expected exact base snapshot.
 * @param layer Expected semantic layer.
 * @param feature Expected semantic feature.
 * @param path Expected project-relative source path.
 * @throws When the candidate response targets a different semantic scope.
 */
function verifyCandidate(
  response: GeoGeometryEditResponse | GeoAttributeEditResponse,
  project: string,
  snapshot: string,
  layer: string,
  feature: string,
  path: string,
): void {
  if (
    response.project !== project ||
    response.baseSnapshot !== snapshot ||
    response.layer !== layer ||
    response.feature !== feature ||
    response.path !== path
  ) {
    throw new TypeError("geospatial candidate identity does not match the requested target");
  }
}

/** Checks that a typed value matches its authored scalar field type.
 * @param value Replacement value.
 * @param fieldType Authored field type.
 * @returns Whether the value is valid for the field.
 */
function valueMatchesField(
  value: GeoValue,
  fieldType: GeoLayerView["fields"][number]["fieldType"],
): boolean {
  return (
    (value.kind === "text" && fieldType === "text") ||
    (value.kind === "number" && fieldType === "number") ||
    (value.kind === "boolean" && fieldType === "boolean") ||
    (value.kind === "date_time" && fieldType === "date_time")
  );
}

/** Validates ordered bounds and a declared CRS without converting decimal text.
 * @param bounds Requested query envelope.
 * @throws When the bounds are malformed, reversed, or lack CRS identity.
 */
function validateBounds(bounds: GeoFeatureBounds): void {
  const minX = decimalParts(bounds.minX);
  const minY = decimalParts(bounds.minY);
  const maxX = decimalParts(bounds.maxX);
  const maxY = decimalParts(bounds.maxY);
  if (compareDecimal(minX, maxX) > 0 || compareDecimal(minY, maxY) > 0) {
    throw new RangeError("spatial query minimum bounds must not exceed maximum bounds");
  }
  if (
    ![bounds.crs.authority, bounds.crs.code, bounds.crs.definition].some((part) => part?.trim())
  ) {
    throw new TypeError("spatial query bounds require an explicit CRS");
  }
}

/** A canonical base-ten decimal split into sign, integer, and fractional parts. */
type DecimalParts = [boolean, string, string];

/** Parses exact base-ten coordinate text into sign, integer, and fractional parts.
 * @param value Coordinate text to parse.
 * @returns Canonical decimal components.
 * @throws When coordinate text is not a finite decimal string.
 */
function decimalParts(value: string): DecimalParts {
  const match = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(value);
  if (match === null) throw new TypeError("spatial query bounds must be decimal strings");
  const integer = (match[2] ?? "0").replace(/^0+(?=\d)/, "");
  const fraction = (match[3] ?? "").replace(/0+$/, "");
  const negative = match[1] === "-" && /[1-9]/.test(integer + fraction);
  return [negative, integer, fraction];
}

/** Compares two canonical base-ten coordinate values exactly.
 * @param left First decimal components.
 * @param right Second decimal components.
 * @returns Negative, zero, or positive according to numeric ordering.
 */
function compareDecimal(left: DecimalParts, right: DecimalParts): number {
  if (left[0] !== right[0]) return left[0] ? -1 : 1;
  const sign = left[0] ? -1 : 1;
  const integerOrder =
    left[1].length - right[1].length || (left[1] < right[1] ? -1 : left[1] > right[1] ? 1 : 0);
  if (integerOrder !== 0) return sign * integerOrder;
  const precision = Math.max(left[2].length, right[2].length);
  const leftFraction = left[2].padEnd(precision, "0");
  const rightFraction = right[2].padEnd(precision, "0");
  return sign * (leftFraction < rightFraction ? -1 : leftFraction > rightFraction ? 1 : 0);
}
