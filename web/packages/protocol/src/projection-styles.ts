/** @file Defines portable projection style schema, resolution, and setting contracts. */

/** Purpose that controls which projection style sources may apply. */
export type ProjectionStylePurpose = "interactiveView" | "sharedMaterialization" | "explicitExport";

/** Configuration scope targeted by a style setting. */
export type ProjectionStyleScope =
  | { kind: "user" }
  | { kind: "view"; viewId: string }
  | { kind: "project"; projectId: string }
  | { kind: "workspace"; workspaceId: string };

/** Requests the style schema advertised for a projection kind. */
export interface ProjectionStyleSchemaRequest {
  /** Projection kind whose adapter schema is requested. */
  projectionKind: string;
}

/** Requests purpose-aware style resolution. */
export interface ProjectionStyleResolveRequest {
  /** Projection kind being resolved. */
  projectionKind: string;
  /** Stable project identity for project policy and conventions. */
  projectId: string;
  /** Stable workspace identity for materialization policy. */
  workspaceId: string;
  /** Purpose controlling eligible style sources. */
  purpose: ProjectionStylePurpose;
  /** View identity when resolving an interactive view. */
  viewId?: string;
  /** Explicit style bytes for an export operation. */
  explicitPersonal?: number[];
}

/** Requests a style setting write or clear in an explicit scope. */
export interface ProjectionStyleSettingRequest {
  /** Configuration scope selected by the caller. */
  scope: ProjectionStyleScope;
  /** Projection kind affected by the setting. */
  projectionKind: string;
  /** Expected revision for compare-and-swap when supplied. */
  expectedRevision?: number;
  /** Canonical adapter style bytes, or undefined to clear the setting. */
  style?: number[];
}

/** Describes a domain-owned style schema. */
export interface ProjectionStyleSchemaResponse {
  /** Stable adapter or schema identity. */
  reference: { adapterId: string; version: string };
  /** Projection kinds understood by the schema. */
  projectionKinds: string[];
  /** Ordered generic property descriptors. */
  properties: ProjectionStyleProperty[];
}

/** Describes one generic property without assigning it domain semantics. */
export interface ProjectionStyleProperty {
  /** Stable adapter-owned property key. */
  key: string;
  /** Human-readable label. */
  label: string;
  /** Optional safe description. */
  description?: string;
  /** Generic value shape. */
  kind: "boolean" | "integer" | "enum" | "text" | "opaque";
  /** Allowed enum values. */
  allowedValues: string[];
  /** Inclusive lower integer bound. */
  minimum?: number;
  /** Inclusive upper integer bound. */
  maximum?: number;
  /** Canonical encoded default bytes. */
  default: number[];
  /** Projection purposes this property may affect. */
  applicability: "viewOnly" | "materializationOnly" | "both";
  /** Whether changing this property can change fidelity. */
  affectsFidelity: boolean;
  /** Whether materialization must retain this property. */
  requiredForMaterialization: boolean;
}

/** Identifies the source reference for resolved style bytes. */
export type ProjectionStyleReference =
  { kind: "inline" } | { kind: "revision"; entity: string; revision: string };

/** Reports fidelity of one exact style and representation. */
export interface ProjectionStyleFidelityReport {
  /** Exact editability category. */
  fidelity: "losslessEditable" | "lossAwareEditable" | "readOnlyLossy";
  /** Semantic categories omitted from the representation. */
  omittedCategories: string[];
  /** Optional safe adapter explanation. */
  explanation?: string;
}

/** Explains one source that contributed to style resolution. */
export interface ProjectionStyleSource {
  /** Stable category such as `domain-default` or `user-preference`. */
  kind: string;
  /** Stable caller-owned source identity. */
  identity: string;
  /** Immutable semantic revision when available. */
  revision?: string;
  /** Mutable configuration generation when available. */
  generation?: number;
  /** Exact style bytes contributed by this source. */
  value: number[];
}

/** Returns effective style, provenance, and fidelity. */
export interface ProjectionStyleResolveResponse {
  /** Purpose used for resolution. */
  purpose: ProjectionStylePurpose;
  /** Resolved style reference. */
  reference: ProjectionStyleReference;
  /** Exact canonical bytes consumed by a renderer. */
  value: number[];
  /** Optional domain schema identity. */
  schema?: { adapterId: string; version: string };
  /** Ordered sources applied by shared resolution. */
  applied: ProjectionStyleSource[];
  /** Fidelity for this exact resolved style. */
  fidelity: ProjectionStyleFidelityReport;
}

/** Returns the authoritative stored style and revision after a mutation. */
export interface ProjectionStyleSettingResponse {
  /** Stored bytes, absent after clearing the setting. */
  style?: number[];
  /** Monotonic setting revision, absent when cleared. */
  revision?: number;
}
