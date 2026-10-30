/** @file Defines typed exact-snapshot CAD line candidate contracts. */
/** Represents one exact decimal coordinate tuple. */
export interface CadCoordinate {
  /** Exact decimal x ordinate. */
  x: string;
  /** Exact decimal y ordinate. */
  y: string;
  /** Exact decimal z ordinate. */
  z: string;
}

/** Requests one exact-snapshot CAD drawing summary. */
export interface CadDrawingInspectRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
}

/** Describes one semantic drawing layer. */
export interface CadLayerSummary {
  /** Stable semantic layer identity. */
  id: string;
  /** Authored layer name. */
  name: string;
  /** Whether the layer is visible. */
  visible: boolean;
  /** Whether the layer is locked. */
  locked: boolean;
}

/** Returns CAD drawing metadata and its layers. */
export interface CadDrawingInspectResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot used for all records. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
  /** Authored or adapter-reported unit label. */
  units: string;
  /** Optional coordinate reference label. */
  coordinateReference?: string;
  /** Ordered semantic layers. */
  layers: CadLayerSummary[];
  /** Number of supported imported entities. */
  entityCount: number;
  /** Whether source content fully mapped to current semantics. */
  complete: boolean;
  /** Safe import and fidelity diagnostics. */
  diagnostics: string[];
}

/** Requests one bounded CAD entity page. */
export interface CadEntitiesRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
  /** Zero-based source-order offset. */
  offset: number;
  /** Requested page size, at most five hundred. */
  limit: number;
}

/** Describes a semantic CAD entity without parser-native fields. */
export interface CadEntitySummary {
  /** Stable semantic entity identity. */
  id: string;
  /** Stable semantic layer identity. */
  layer: string;
  /** Model, paper, or named-space membership. */
  space: CadDrawingSpace;
  /** Typed semantic geometry. */
  geometry: CadGeometry;
  /** Optional adapter-neutral style reference. */
  style?: CadEntityStyleRef;
}

/** Identifies a CAD style without embedding adapter internals. */
export interface CadEntityStyleRef {
  /** Adapter-owned style key. */
  key: string;
  /** Adapter resolving the style. */
  adapter: string;
}

/** Represents model space, paper space, or an adapter-defined named space. */
export type CadDrawingSpace = "Model" | { Paper: string } | { Named: string };

/** Represents one format-independent semantic CAD geometry variant. */
export type CadGeometry =
  | { Line: { start: CadCoordinate; end: CadCoordinate } }
  | { Polyline: { vertices: CadCoordinate[]; closed: boolean } }
  | { Circle: { center: CadCoordinate; radius: string } }
  | { Arc: { center: CadCoordinate; radius: string; start_angle: string; end_angle: string } }
  | { Text: { position: CadCoordinate; value: string; height: string } }
  | { Dimension: { definition_points: CadCoordinate[]; text?: string } }
  | {
      BlockReference: {
        definition: string;
        insertion: CadCoordinate;
        scale: [string, string, string];
        rotation: string;
      };
    };

/** Returns a bounded CAD entity page. */
export interface CadEntitiesResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot used for all records. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
  /** Bounded semantic entities. */
  entities: CadEntitySummary[];
  /** Total entity count. */
  total: number;
  /** Whether more entities remain. */
  hasMore: boolean;
}

/** Defines a two-dimensional CAD selection envelope in drawing units. */
export interface CadBounds2D {
  /** Minimum x ordinate as decimal text. */
  minX: string;
  /** Minimum y ordinate as decimal text. */
  minY: string;
  /** Maximum x ordinate as decimal text. */
  maxX: string;
  /** Maximum y ordinate as decimal text. */
  maxY: string;
}

/** Requests entities intersecting bounds in one exact CAD drawing space. */
export interface CadBoundsQueryRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
  /** Model, paper, or named drawing space to query. */
  space: CadDrawingSpace;
  /** Two-dimensional selection envelope in drawing units. */
  bounds: CadBounds2D;
  /** Zero-based result offset. */
  offset: number;
  /** Requested page size, at most five hundred. */
  limit: number;
}

/** Returns one bounded CAD bounds query page. */
export interface CadBoundsQueryResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot used for the query. */
  snapshot: string;
  /** Stable semantic drawing identity. */
  drawing: string;
  /** CAD coordinate space queried. */
  space: CadDrawingSpace;
  /** Exact decimal-text selection envelope. */
  bounds: CadBounds2D;
  /** Matching semantic entities in source order. */
  entities: CadEntitySummary[];
  /** Total matches before pagination. */
  total: number;
  /** Whether another page remains. */
  hasMore: boolean;
  /** Number of queried-space entities with invalid or unsupported extents. */
  unboundedEntities: number;
  /** Whether each entity in the queried space had a calculable extent. */
  complete: boolean;
}

/** Requests a semantic line edit candidate. */
export interface CadLineEditRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic CAD line identity. */
  entity: string;
  /** Replacement start point. */
  start: CadCoordinate;
  /** Replacement end point. */
  end: CadCoordinate;
}

/** Returns a reviewable line candidate handle. */
export interface CadLineEditResponse {
  /** Candidate identity used by shared review operations. */
  candidateId: string;
  /** Project owning the candidate. */
  project: string;
  /** Exact base snapshot used by candidate creation. */
  baseSnapshot: string;
  /** Project-relative DXF source path. */
  path: string;
  /** Candidate validation state. */
  validation:
    { status: "pending" } | { status: "passed" } | { status: "failed"; diagnostics: string[] };
}

/** Requests a semantic entity layer change candidate. */
export interface CadLayerChangeRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic CAD entity identity. */
  entity: string;
  /** Existing semantic layer in the same drawing. */
  destinationLayer: string;
}

/** Returns a reviewable layer-change candidate handle. */
export type CadLayerChangeResponse = CadLineEditResponse;
