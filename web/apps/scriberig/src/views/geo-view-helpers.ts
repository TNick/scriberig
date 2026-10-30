/** @file Defines exact-snapshot helpers for geospatial and QGIS views. */
import type {
  GeoFeatureQueryResponse,
  GeoFeatureView,
  GeoFeaturesResponse,
  GeoLayerView,
  QgisProjectResponse,
} from "@scriberig/protocol";

/** Builds a typed envelope from a layer's source extent and CRS.
 * @param layer Exact semantic layer summary.
 * @returns Explicit CRS-bound spatial query window.
 * @throws When extent or CRS metadata is unavailable.
 */
export function extentBounds(layer: GeoLayerView) {
  if (layer.extent === undefined || layer.crs === undefined) {
    throw new TypeError("layer has no explicit extent and CRS for a spatial query");
  }
  return {
    minX: layer.extent[0],
    minY: layer.extent[1],
    maxX: layer.extent[2],
    maxY: layer.extent[3],
    crs: layer.crs,
  };
}

/** Returns bounded diagnostics when the page came from a spatial predicate query.
 * @param page Exact-snapshot GeoPackage feature result.
 * @returns Safe query diagnostics or an empty array for ordinary feature pages.
 */
export function pageDiagnostics(page: GeoFeaturesResponse | GeoFeatureQueryResponse): string[] {
  return "sourceRevision" in page ? page.diagnostics : [];
}

/** Checks whether a QGIS reference resolves to the GeoPackage reader surface.
 * @param reference Authored QGIS layer reference.
 * @returns Whether its linked source can be opened by the current semantic map view.
 */
export function qgisReferenceIsBrowsable(
  reference: QgisProjectResponse["layers"][number],
): boolean {
  return (
    reference.layer !== undefined &&
    reference.resolvedPath?.toLocaleLowerCase().endsWith(".gpkg") === true
  );
}

/** Formats one typed attribute for an editable text control.
 * @param feature Selected semantic feature.
 * @param field Authored field name.
 * @returns Editable scalar text.
 */
export function attributeText(feature: GeoFeatureView, field: string): string {
  const item = feature.attributes[field];
  if (item === undefined || item.kind === "null") return "";
  return String(item.value);
}
