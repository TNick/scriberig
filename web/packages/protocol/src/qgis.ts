/** @file Defines provider-neutral QGIS project and layer-reference views. */
import type { GeoCrsView } from "./geo";

/** Requests an exact-snapshot QGIS project inspection. */
export interface QgisProjectRequest {
  /** Stable owning project identity. */
  project: string;
  /** Exact project-relative QGS or QGZ source path. */
  path: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
}

/** Requests an exact-snapshot QGIS layer-reference rename candidate. */
export interface QgisLayerRenameRequest {
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Stable semantic QGIS layer-reference identity. */
  reference: string;
  /** New authored display name. */
  newName: string;
}

/** Returns the shared review handle for a QGIS layer-reference rename. */
export interface QgisLayerRenameResponse {
  /** Stable project candidate identity. */
  candidate: string;
  /** Stable owning project identity. */
  project: string;
  /** Exact candidate base snapshot. */
  baseSnapshot: string;
  /** Stable semantic QGIS layer-reference identity. */
  reference: string;
  /** Exact project-relative QGS or QGZ path. */
  path: string;
}

/** Requests a complete exact-snapshot QGIS project layer-order candidate. */
export interface QgisLayerOrderRequest {
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Exact project-relative QGS or QGZ source path. */
  path: string;
  /** Complete permutation of semantic layer-reference identities. */
  orderedReferences: string[];
}

/** Returns the shared review handle for a QGIS project layer-order edit. */
export interface QgisLayerOrderResponse {
  /** Stable project candidate identity. */
  candidate: string;
  /** Stable owning project identity. */
  project: string;
  /** Exact candidate base snapshot. */
  baseSnapshot: string;
  /** Exact project-relative QGS or QGZ path. */
  path: string;
}

/** Describes one QGIS layer reference without copying its dataset content. */
export interface QgisLayerReferenceView {
  /** Stable semantic reference identity. */
  id: string;
  /** Authored display name. */
  name: string;
  /** Exact authored datasource string. */
  dataSource: string;
  /** Authored project-relative path, when present. */
  path?: string;
  /** Resolved project-relative source path, when known. */
  resolvedPath?: string;
  /** Authored source layer/table name, when present. */
  sourceLayer?: string;
  /** Resolved semantic dataset identity, when available. */
  dataset?: string;
  /** Resolved shared GeoLayer identity, when available. */
  layer?: string;
  /** Safe explanation when reference resolution failed. */
  resolutionDiagnostic?: string;
  /** Authored display visibility. */
  visible: boolean;
  /** Authored renderer kind when supported. */
  renderer?: string;
  /** Authored map scale range when present. */
  scaleRange?: string;
}

/** Describes one QGIS layer group and its ordered references. */
export interface QgisLayerGroupView {
  /** Stable semantic group identity. */
  id: string;
  /** Authored group name. */
  name: string;
  /** Ordered QGIS reference identities. */
  layers: string[];
}

/** Returns QGIS project metadata and its separately identified layer references. */
export interface QgisProjectResponse {
  /** Stable owning project identity. */
  project: string;
  /** Exact project-relative source path. */
  path: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Semantic revision of the QGIS project model. */
  revision: string;
  /** Stable QGIS project identity. */
  id: string;
  /** Authored project title. */
  title?: string;
  /** Explicit project CRS when present. */
  crs?: GeoCrsView;
  /** Ordered project layer references. */
  layers: QgisLayerReferenceView[];
  /** Project groups and their ordered reference identities. */
  groups: QgisLayerGroupView[];
}
