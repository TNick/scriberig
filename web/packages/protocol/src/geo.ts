/** @file Defines provider-neutral geospatial feature read contracts. */

/** Requests a bounded GeoPackage feature page at an exact snapshot. */
export interface GeoFeaturesRequest {
  /** Stable owning project identity. */
  project: string;
  /** Project-relative source path from the artifact manifest. */
  path: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Zero-based feature offset. */
  offset: number;
  /** Maximum feature count, from one through one thousand. */
  limit: number;
}

/** Requests one bounded typed spatial query in an explicit coordinate system. */
export interface GeoFeatureQueryRequest {
  /** Stable owning project identity. */
  project: string;
  /** Project-relative source path from the artifact manifest. */
  path: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Typed attribute or envelope predicate. */
  predicate: GeoFeaturePredicate;
  /** Zero-based matching feature offset. */
  offset: number;
  /** Maximum number of matches, from one through one thousand. */
  limit: number;
}

/** Selects semantic features using typed equality or a CRS-bound envelope. */
export type GeoFeaturePredicate =
  | { kind: "attribute_equals"; field: string; value: GeoValue }
  | { kind: "envelope_overlaps"; bounds: GeoFeatureBounds };

/** Describes an axis-aligned envelope with its coordinate reference system. */
export interface GeoFeatureBounds {
  /** Minimum x/easting decimal text. */
  minX: string;
  /** Minimum y/northing decimal text. */
  minY: string;
  /** Maximum x/easting decimal text. */
  maxX: string;
  /** Maximum y/northing decimal text. */
  maxY: string;
  /** Explicit bounds coordinate reference system. */
  crs: GeoCrsView;
}

/** Requests a bounded dataset and layer catalog page. */
export interface GeoCatalogRequest {
  /** Stable owning project identity. */
  project: string;
  /** Project-relative source path from the artifact manifest. */
  path: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Zero-based layer offset. */
  offset: number;
  /** Maximum layer count, from one through two hundred. */
  limit: number;
}

/** Returns bounded layer metadata without feature rows. */
export interface GeoCatalogResponse {
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Stable semantic dataset identity. */
  dataset: string;
  /** BLAKE3 digest of exact GeoPackage bytes. */
  sourceDigest: string;
  /** Ordered semantic layer page. */
  layers: GeoLayerView[];
  /** Total source layer count. */
  total: number;
  /** Whether another layer page is available. */
  hasMore: boolean;
}

/** Represents one semantic layer schema and fidelity summary. */
export interface GeoLayerView {
  /** Stable semantic layer identity. */
  id: string;
  /** Stable dataset identity. */
  dataset: string;
  /** Authored layer name. */
  name: string;
  /** Ordered authored attribute fields. */
  fields: GeoFieldView[];
  /** Explicit CRS metadata, when available. */
  crs?: GeoCrsView;
  /** Declared geometry kind, when available. */
  geometryType?: "point" | "line_string" | "polygon" | "geometry_collection";
  /** Source feature count without row enumeration. */
  featureCount: number;
  /** Authored extent in exact decimal text. */
  extent?: [string, string, string, string];
  /** Whether semantic editing is currently supported. */
  editable: boolean;
  /** Safe provider/fidelity diagnostics. */
  diagnostics: string[];
}

/** Represents one typed layer attribute field. */
export interface GeoFieldView {
  /** Authored field name. */
  name: string;
  /** Provider-neutral scalar field type. */
  fieldType: "text" | "number" | "boolean" | "date_time" | "binary";
  /** Whether null values are allowed. */
  nullable: boolean;
}

/** Represents explicit CRS authority and definition metadata. */
export interface GeoCrsView {
  /** Authority namespace such as EPSG. */
  authority?: string;
  /** Authority code when known. */
  code?: string;
  /** Exact source CRS definition. */
  definition?: string;
}

/** Returns a bounded feature page with exact semantic context. */
export interface GeoFeaturesResponse {
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable semantic snapshot identity. */
  snapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Ordered provider-neutral feature page. */
  features: GeoFeatureView[];
  /** Whether another bounded page is available. */
  hasMore: boolean;
}

/** Represents one provider-neutral feature. */
export interface GeoFeatureView {
  /** Stable Scriberig feature identity. */
  id: string;
  /** Native provider identity retained as source evidence. */
  nativeId?: string;
  /** Typed geometry in the layer's explicit CRS. */
  geometry?: GeoGeometry;
  /** Authored typed feature attributes. */
  attributes: Record<string, GeoValue>;
}

/** Returns exact-source provenance for a bounded spatial query result. */
export interface GeoFeatureQueryResponse extends GeoFeaturesResponse {
  /** Exact source revision used for query evaluation. */
  sourceRevision: string;
  /** BLAKE3 digest of exact source package bytes. */
  sourceDigest: string;
  /** Provider adapter used to read the source. */
  provider: string;
  /** Coordinate reference system used by the selected layer. */
  layerCrs?: GeoCrsView;
  /** Stable name of the bounded evaluation strategy. */
  strategy: string;
  /** Optional coordinate transformation record. */
  coordinateTransform?: string;
  /** Safe query diagnostics. */
  diagnostics: string[];
}

/** Requests one exact-base feature geometry candidate. */
export interface GeoGeometryEditRequest {
  /** Stable owning project identity. */
  project: string;
  /** Exact project-relative GeoPackage path. */
  path: string;
  /** Exact immutable base snapshot identity. */
  snapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Stable semantic feature identity. */
  feature: string;
  /** Replacement geometry expressed in the selected layer CRS. */
  geometry: GeoGeometry;
}

/** Returns one reviewable geometry candidate handle. */
export interface GeoGeometryEditResponse {
  /** Durable candidate identity. */
  candidate: string;
  /** Stable owning project identity. */
  project: string;
  /** Exact base snapshot identity. */
  baseSnapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Stable semantic feature identity. */
  feature: string;
  /** Exact project-relative source path. */
  path: string;
}

/** Requests one exact-base typed feature attribute candidate. */
export interface GeoAttributeEditRequest {
  /** Stable owning project identity. */
  project: string;
  /** Exact project-relative GeoPackage path. */
  path: string;
  /** Exact immutable base snapshot identity. */
  snapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Stable semantic feature identity. */
  feature: string;
  /** Authored schema field name. */
  field: string;
  /** Typed replacement value. */
  value: GeoValue;
}

/** Returns one reviewable attribute candidate handle. */
export interface GeoAttributeEditResponse {
  /** Durable candidate identity. */
  candidate: string;
  /** Stable owning project identity. */
  project: string;
  /** Exact base snapshot identity. */
  baseSnapshot: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Stable semantic feature identity. */
  feature: string;
  /** Authored schema field name. */
  field: string;
  /** Exact project-relative source path. */
  path: string;
}

/** Represents provider-neutral geometry with exact decimal ordinates. */
export type GeoGeometry =
  | { kind: "point"; coordinates: GeoCoordinate }
  | { kind: "line_string"; coordinates: GeoCoordinate[] }
  | { kind: "polygon"; coordinates: GeoCoordinate[][] }
  | { kind: "collection"; coordinates: GeoGeometry[] };

/** Represents exact coordinate values in the layer CRS. */
export interface GeoCoordinate {
  /** Exact x/easting decimal text. */
  x: string;
  /** Exact y/northing decimal text. */
  y: string;
  /** Optional exact z ordinate. */
  z?: string;
  /** Optional exact measure ordinate. */
  measure?: string;
}

/** Represents one typed feature attribute value. */
export type GeoValue =
  | { kind: "null" }
  | { kind: "text"; value: string }
  | { kind: "number"; value: string }
  | { kind: "boolean"; value: boolean }
  | { kind: "date_time"; value: string };
