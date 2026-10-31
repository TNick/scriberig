/** @file Provides rich artifact and projection style operations for the semantic client. */
import type {
  CadBoundsQueryRequest,
  CadBoundsQueryResponse,
  CadDrawingInspectRequest,
  CadDrawingInspectResponse,
  CadEntitiesRequest,
  CadEntitiesResponse,
  CadLayerChangeRequest,
  CadLayerChangeResponse,
  CadLineEditRequest,
  CadLineEditResponse,
  DocumentInspectRequest,
  DocumentInspectResponse,
  DocumentParagraphEditRequest,
  DocumentParagraphEditResponse,
  DocumentParagraphInsertRequest,
  DocumentParagraphInsertResponse,
  DocumentTableCellEditRequest,
  GeoAttributeEditRequest,
  GeoAttributeEditResponse,
  GeoCatalogRequest,
  GeoCatalogResponse,
  GeoFeatureQueryRequest,
  GeoFeatureQueryResponse,
  GeoFeaturesRequest,
  GeoFeaturesResponse,
  GeoGeometryEditRequest,
  GeoGeometryEditResponse,
  ProjectionStyleResolveRequest,
  ProjectionStyleResolveResponse,
  ProjectionStyleSchemaResponse,
  ProjectionStyleSettingRequest,
  ProjectionStyleSettingResponse,
  QgisLayerOrderRequest,
  QgisLayerOrderResponse,
  QgisLayerRenameRequest,
  QgisLayerRenameResponse,
  QgisProjectRequest,
  QgisProjectResponse,
} from "@scriberig/protocol";
import {
  normalizeCadBoundsQueryResponse,
  normalizeCadDrawingInspectResponse,
  normalizeCadEntitiesResponse,
  normalizeCadLineEditResponse,
} from "./cad-views";
import {
  normalizeDocumentInspectResponse,
  normalizeDocumentParagraphEditResponse,
  normalizeDocumentParagraphInsertResponse,
} from "./document-views";
import {
  normalizeGeoAttributeEditResponse,
  normalizeGeoGeometryEditResponse,
} from "./geo-edit-views";
import {
  normalizeGeoCatalogResponse,
  normalizeGeoFeatureQueryResponse,
  normalizeGeoFeaturesResponse,
} from "./geo-views";
import {
  normalizeProjectionStyleResolution,
  normalizeProjectionStyleSchema,
  normalizeProjectionStyleSetting,
} from "./projection-style-views";
import {
  normalizeQgisLayerOrderResponse,
  normalizeQgisLayerRenameResponse,
  normalizeQgisProjectResponse,
} from "./qgis-views";
import { SpreadsheetClientOperations } from "./spreadsheet-client-operations";

/** Owns reusable rich artifact reads, edits, and projection style operations.
 *
 * Public members: `projectionStyleSchema`, `resolveProjectionStyle`, `setProjectionStyle`,
 * `inspectGeoPackage`, `geoFeatures`, `queryGeoFeatures`, `proposeGeoGeometryEdit`,
 * `proposeGeoAttributeEdit`, `inspectQgisProject`, `proposeQgisLayerRename`,
 * `proposeQgisLayerOrder`, `proposeCadLineEdit`, `proposeCadLayerChange`, `inspectCadDrawing`,
 * `cadEntities`, `cadEntitiesIntersectingBounds`, `proposeDocumentParagraphEdit`,
 * `proposeDocumentTableCellEdit`, `proposeDocumentParagraphInsert`, and `inspectDocument`.
 * Private members: inherited semantic transport.
 */
export class RichArtifactClientOperations extends SpreadsheetClientOperations {
  /** Reads the adapter-owned schema for one projection kind.
   * @param projectionKind Stable projection kind such as `javascript`.
   * @param signal Optional request cancellation signal.
   * @returns The schema when a domain adapter declares one.
   */
  projectionStyleSchema(
    projectionKind: string,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleSchemaResponse | undefined> {
    return this.transport
      .request<unknown>("projection_style_schema", { projectionKind }, signal)
      .then(normalizeProjectionStyleSchema);
  }
  /** Resolves effective style, source provenance, and fidelity.
   * @param request Exact style purpose and project/workspace/view scopes.
   * @param signal Optional request cancellation signal.
   * @returns Exact style bytes with contributing sources and editability.
   */
  resolveProjectionStyle(
    request: ProjectionStyleResolveRequest,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleResolveResponse> {
    return this.transport
      .request<unknown>("projection_style_resolve", request, signal)
      .then(normalizeProjectionStyleResolution);
  }
  /** Sets or clears one style setting through the shared application semantics.
   * @param request Explicit scope, projection kind, revision, and optional bytes.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative stored style and revision.
   */
  setProjectionStyle(
    request: ProjectionStyleSettingRequest,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleSettingResponse> {
    return this.transport
      .request<unknown>("projection_style_setting", request, signal)
      .then(normalizeProjectionStyleSetting);
  }
  /** Reads bounded GeoPackage layer metadata at an exact snapshot.
   * @param request Exact project, source path, snapshot, and page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Dataset identity, source digest, and typed layer summaries.
   */
  inspectGeoPackage(request: GeoCatalogRequest, signal?: AbortSignal): Promise<GeoCatalogResponse> {
    return this.transport.request("geo_catalog", request, signal).then(normalizeGeoCatalogResponse);
  }
  /** Reads a bounded GeoPackage feature page at an exact snapshot.
   * @param request Exact project, source path, snapshot, layer, and page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Typed features with provider-neutral geometry and attributes.
   */
  geoFeatures(request: GeoFeaturesRequest, signal?: AbortSignal): Promise<GeoFeaturesResponse> {
    return this.transport
      .request("geo_features", request, signal)
      .then(normalizeGeoFeaturesResponse);
  }
  /** Queries one exact layer by typed predicate with bounded paging.
   * @param request Exact project, source, snapshot, predicate, and bounds.
   * @param signal Optional request cancellation signal.
   * @returns Matching features with exact source provenance.
   */
  queryGeoFeatures(
    request: GeoFeatureQueryRequest,
    signal?: AbortSignal,
  ): Promise<GeoFeatureQueryResponse> {
    return this.transport
      .request("geo_feature_query", request, signal)
      .then(normalizeGeoFeatureQueryResponse);
  }
  /** Proposes one geometry edit through the shared candidate workflow.
   * @param request Exact project, source, snapshot, target, and replacement geometry.
   * @param signal Optional request cancellation signal.
   * @returns Shared candidate handle and exact target identities.
   */
  proposeGeoGeometryEdit(
    request: GeoGeometryEditRequest,
    signal?: AbortSignal,
  ): Promise<GeoGeometryEditResponse> {
    return this.transport
      .request("geo_geometry_edit", request, signal)
      .then(normalizeGeoGeometryEditResponse);
  }
  /** Proposes one typed attribute edit through the shared candidate workflow.
   * @param request Exact project, source, snapshot, target, field, and value.
   * @param signal Optional request cancellation signal.
   * @returns Shared candidate handle and exact target identities.
   */
  proposeGeoAttributeEdit(
    request: GeoAttributeEditRequest,
    signal?: AbortSignal,
  ): Promise<GeoAttributeEditResponse> {
    return this.transport
      .request("geo_attribute_edit", request, signal)
      .then(normalizeGeoAttributeEditResponse);
  }
  /** Inspects authored QGIS layer references and their resolved GeoLayer links.
   * @param request Exact project, path, and snapshot identity.
   * @param signal Optional request cancellation signal.
   * @returns Typed project view with distinct reference and dataset identities.
   */
  inspectQgisProject(
    request: QgisProjectRequest,
    signal?: AbortSignal,
  ): Promise<QgisProjectResponse> {
    return this.transport
      .request("qgis_project", request, signal)
      .then(normalizeQgisProjectResponse);
  }
  /** Proposes a QGIS layer-reference rename through the shared semantic path.
   * @param request Exact project, snapshot, reference, and new authored name.
   * @param signal Optional request cancellation signal.
   * @returns Candidate identity for shared review and apply.
   */
  proposeQgisLayerRename(
    request: QgisLayerRenameRequest,
    signal?: AbortSignal,
  ): Promise<QgisLayerRenameResponse> {
    return this.transport
      .request("qgis_layer_rename", request, signal)
      .then(normalizeQgisLayerRenameResponse);
  }
  /** Proposes a complete QGIS layer ordering through the shared semantic path.
   * @param request Exact project, snapshot, path, and complete layer permutation.
   * @param signal Optional request cancellation signal.
   * @returns Candidate identity for shared review and apply.
   */
  proposeQgisLayerOrder(
    request: QgisLayerOrderRequest,
    signal?: AbortSignal,
  ): Promise<QgisLayerOrderResponse> {
    return this.transport
      .request("qgis_layer_order", request, signal)
      .then(normalizeQgisLayerOrderResponse);
  }
  /** Creates a line geometry candidate through the shared semantic edit path.
   * @param request Exact project/snapshot/entity and replacement coordinates.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeCadLineEdit(
    request: CadLineEditRequest,
    signal?: AbortSignal,
  ): Promise<CadLineEditResponse> {
    return this.transport
      .request("cad_line_edit", request, signal)
      .then(normalizeCadLineEditResponse);
  }
  /** Creates a semantic layer-change candidate through the shared edit path.
   * @param request Exact project/snapshot/entity and existing destination layer.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeCadLayerChange(
    request: CadLayerChangeRequest,
    signal?: AbortSignal,
  ): Promise<CadLayerChangeResponse> {
    return this.transport
      .request("cad_layer_change", request, signal)
      .then(normalizeCadLineEditResponse);
  }
  /** Reads a snapshot-bound semantic CAD drawing summary.
   * @param request Exact project/snapshot/drawing identities.
   * @param signal Optional request cancellation signal.
   * @returns Drawing metadata, layers, and fidelity diagnostics.
   */
  inspectCadDrawing(
    request: CadDrawingInspectRequest,
    signal?: AbortSignal,
  ): Promise<CadDrawingInspectResponse> {
    return this.transport
      .request("cad_drawing_inspect", request, signal)
      .then(normalizeCadDrawingInspectResponse);
  }
  /** Reads one bounded semantic CAD entity page.
   * @param request Exact drawing snapshot and page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Typed CAD entities and continuation metadata.
   */
  cadEntities(request: CadEntitiesRequest, signal?: AbortSignal): Promise<CadEntitiesResponse> {
    return this.transport
      .request("cad_entities", request, signal)
      .then(normalizeCadEntitiesResponse);
  }
  /** Reads entities intersecting exact bounds in one snapshot and drawing space.
   * @param request Exact snapshot, drawing space, bounds, and page window.
   * @param signal Optional request cancellation signal.
   * @returns Matching entities and extent completeness metadata.
   */
  cadEntitiesIntersectingBounds(
    request: CadBoundsQueryRequest,
    signal?: AbortSignal,
  ): Promise<CadBoundsQueryResponse> {
    return this.transport
      .request("cad_bounds_query", request, signal)
      .then(normalizeCadBoundsQueryResponse);
  }
  /** Creates an exact-base semantic paragraph replacement candidate.
   * @param request Exact paragraph identity and replacement text.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeDocumentParagraphEdit(
    request: DocumentParagraphEditRequest,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphEditResponse> {
    return this.transport
      .request("document_paragraph_edit", request, signal)
      .then(normalizeDocumentParagraphEditResponse);
  }
  /** Creates an exact-base table-cell replacement candidate.
   * @param request Exact table-cell identity and replacement text.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeDocumentTableCellEdit(
    request: DocumentTableCellEditRequest,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphEditResponse> {
    return this.transport
      .request("document_table_cell_edit", request, signal)
      .then(normalizeDocumentParagraphEditResponse);
  }
  /** Creates an exact-base paragraph append candidate for one semantic section.
   * @param request Exact section identity and authored paragraph text.
   * @param signal Optional request cancellation signal.
   * @returns Reviewable candidate identity and validation state.
   */
  proposeDocumentParagraphInsert(
    request: DocumentParagraphInsertRequest,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphInsertResponse> {
    return this.transport
      .request("document_paragraph_insert", request, signal)
      .then(normalizeDocumentParagraphInsertResponse);
  }
  /** Reads the bounded semantic document tree at the requested exact snapshot.
   * @param request Exact project, snapshot, and document identities.
   * @param signal Optional request cancellation signal.
   * @returns Ordered semantic sections, paragraphs, and tables.
   */
  inspectDocument(
    request: DocumentInspectRequest,
    signal?: AbortSignal,
  ): Promise<DocumentInspectResponse> {
    return this.transport
      .request("document_inspect", request, signal)
      .then(normalizeDocumentInspectResponse);
  }
}
