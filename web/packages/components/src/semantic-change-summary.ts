/** @file Builds bounded semantic old-to-new summaries for rich-artifact review. */
import type {
  CadCoordinate,
  GeoFeatureView,
  GeoGeometry,
  GeoValue,
  QgisLayerReferenceView,
} from "@scriberig/protocol";

/** Formats one typed attribute edit for the shared candidate review.
 * @param feature Exact semantic feature being edited.
 * @param field Authored field name.
 * @param value Typed replacement value.
 * @returns Bounded semantic old-to-new attribute summary.
 */
export function summarizeGeoAttributeChange(
  feature: GeoFeatureView,
  field: string,
  value: GeoValue,
): string {
  return `Feature ${feature.nativeId ?? feature.id}, attribute ${boundedSemanticPreview(field)}: ${geoValueLabel(feature.attributes[field])} → ${geoValueLabel(value)}`;
}

/** Formats one geometry edit for the shared candidate review.
 * @param feature Exact semantic feature being edited.
 * @param geometry Typed replacement geometry.
 * @returns Bounded semantic old-to-new geometry summary.
 */
export function summarizeGeoGeometryChange(feature: GeoFeatureView, geometry: GeoGeometry): string {
  const previous = feature.geometry === undefined ? "(none)" : JSON.stringify(feature.geometry);
  return `Feature ${feature.nativeId ?? feature.id}, geometry: ${boundedSemanticPreview(previous)} → ${boundedSemanticPreview(JSON.stringify(geometry))}`;
}

/** Formats a QGIS layer-reference rename for the shared candidate review.
 * @param reference Stable QGIS layer-reference identity.
 * @param previousName Existing authored layer name when available.
 * @param nextName Proposed authored layer name.
 * @returns Bounded semantic old-to-new name summary.
 */
export function summarizeQgisLayerRename(
  reference: string,
  previousName: string | undefined,
  nextName: string,
): string {
  return `QGIS layer reference ${reference} changes name from “${boundedSemanticPreview(previousName ?? "(unknown)")}” to “${boundedSemanticPreview(nextName)}”.`;
}

/** Formats a complete QGIS project layer-order change for shared candidate review.
 * @param previous Ordered project layer references at the base snapshot.
 * @param next Proposed ordered project layer references.
 * @returns Bounded readable semantic before/after order summary.
 */
export function summarizeQgisLayerOrder(
  previous: readonly QgisLayerReferenceView[],
  next: readonly QgisLayerReferenceView[],
): string {
  const names = (layers: readonly QgisLayerReferenceView[]) =>
    boundedSemanticPreview(layers.map((layer) => layer.name).join(" → "));
  return `QGIS project layer order: ${names(previous)} → ${names(next)}.`;
}

/** Formats an exact CAD line endpoint edit for the shared candidate review.
 * @param entity Stable semantic drawing-entity identity.
 * @param previous Authored line endpoints from the exact drawing snapshot.
 * @param next Proposed line endpoints.
 * @returns Bounded semantic old-to-new endpoint summary.
 */
export function summarizeCadLineChange(
  entity: string,
  previous: { start: CadCoordinate; end: CadCoordinate },
  next: { start: CadCoordinate; end: CadCoordinate },
): string {
  return `CAD entity ${entity}, line endpoints: start ${coordinateLabel(previous.start)} → ${coordinateLabel(next.start)}; end ${coordinateLabel(previous.end)} → ${coordinateLabel(next.end)}`;
}

/** Formats a CAD entity's layer reassignment for the shared candidate review.
 * @param entity Stable semantic drawing-entity identity.
 * @param previousLayer Authored layer name before the candidate.
 * @param nextLayer Proposed authored layer name.
 * @returns Semantic old-to-new layer summary.
 */
export function summarizeCadLayerChange(
  entity: string,
  previousLayer: string,
  nextLayer: string,
): string {
  return `CAD entity ${entity}, layer: ${boundedSemanticPreview(previousLayer)} → ${boundedSemanticPreview(nextLayer)}`;
}

/** Formats one typed attribute value without losing its semantic kind.
 * @param value Existing or proposed typed value.
 * @returns Readable value with kind, bounded to the review preview size.
 */
function geoValueLabel(value: GeoValue | undefined): string {
  if (value === undefined) return "(absent)";
  const text = value.kind === "null" ? "null" : String(value.value);
  return `${value.kind} ${boundedSemanticPreview(text)}`;
}

/** Formats one exact decimal CAD coordinate without numeric conversion.
 * @param coordinate Authored exact coordinate.
 * @returns Bounded decimal tuple retaining exact ordinate text.
 */
function coordinateLabel(coordinate: CadCoordinate): string {
  return `(${boundedSemanticPreview(coordinate.x)}, ${boundedSemanticPreview(coordinate.y)}, ${boundedSemanticPreview(coordinate.z)})`;
}

/** Bounds semantic text without splitting Unicode code points.
 * @param value Full value text.
 * @returns At most 120 code points, with an ellipsis when truncated.
 */
export function boundedSemanticPreview(value: string): string {
  const points = Array.from(value);
  return points.length <= 120 ? value : `${points.slice(0, 120).join("")}…`;
}
