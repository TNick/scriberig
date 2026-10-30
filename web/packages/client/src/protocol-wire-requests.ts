/** @file Encodes browser read requests for the pinned Rust protocol serde contract. */

/** Converts one canonical prefixed identifier into its serde byte representation.
 * @param value Canonical ID with its type prefix.
 * @param prefix Expected ID prefix.
 * @param byteLength Required identifier width.
 * @returns The encoded bytes used by the Rust universal query types.
 * @throws When the ID does not match its prefix or fixed-width hexadecimal form.
 */
function idBytes(value: string, prefix: string, byteLength: number): number[] {
  const hex = value.startsWith(prefix) ? value.slice(prefix.length) : "";
  if (!new RegExp(`^[0-9a-fA-F]{${byteLength * 2}}$`).test(hex)) {
    throw new Error(`invalid ${prefix} identifier for the pinned read protocol`);
  }
  return Array.from({ length: byteLength }, (_, index) =>
    Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16),
  );
}

/** Maps browser inspection selectors to the pinned protocol representation.
 * @param payload Browser inspection request.
 * @returns The protocol-compatible inspection request.
 * @throws When the snapshot selector is not recognized.
 */
function inspectionRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as {
    entityId?: unknown;
    snapshot?: { kind?: unknown; snapshotId?: unknown; name?: unknown };
    detail?: unknown;
  };
  const snapshot = request.snapshot;
  if (typeof request.entityId !== "string" || snapshot === undefined) return payload;
  let resolvedSnapshot: unknown;
  if (snapshot.kind === "exact" && typeof snapshot.snapshotId === "string") {
    resolvedSnapshot = { Exact: { id: snapshot.snapshotId } };
  } else if (snapshot.kind === "root" && typeof snapshot.name === "string") {
    resolvedSnapshot = { NamedRoot: snapshot.name };
  } else if (snapshot.kind === "current") {
    resolvedSnapshot = "Current";
  } else {
    throw new Error("unsupported inspection snapshot selector");
  }
  return {
    entity: { id: request.entityId },
    snapshot: resolvedSnapshot,
    detail: request.detail === "deep" ? "Deep" : "Compact",
  };
}

/** Maps a browser child-list request to a bounded universal list query.
 * @param payload Browser child-list request.
 * @returns The protocol-compatible list request.
 * @throws When the request lacks a canonical exact snapshot scope.
 */
function listRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as {
    entityId?: unknown;
    snapshot?: { kind?: unknown; snapshotId?: unknown };
    cursor?: unknown;
  };
  if (
    typeof request.entityId !== "string" ||
    request.snapshot?.kind !== "exact" ||
    typeof request.snapshot.snapshotId !== "string"
  ) {
    throw new Error("list requests require an exact entity snapshot");
  }
  const entity = { bytes: idBytes(request.entityId, "ent_", 16) };
  const snapshot = { bytes: idBytes(request.snapshot.snapshotId, "snp_", 32) };
  return {
    specification: {
      subject: { entity, snapshot },
      collection: { owner: entity, collection_type: "children" },
      page: {
        limit: 100,
        continuation: typeof request.cursor === "string" ? request.cursor : null,
      },
      detail: "Compact",
    },
  };
}

/** Encodes one style purpose as the Rust protocol enum variant.
 * @param value Browser purpose name.
 * @returns The externally tagged Rust enum label.
 * @throws When the purpose is unsupported.
 */
function stylePurpose(value: unknown): string {
  const variants: Record<string, string> = {
    interactiveView: "InteractiveView",
    sharedMaterialization: "SharedMaterialization",
    explicitExport: "ExplicitExport",
  };
  if (typeof value !== "string" || variants[value] === undefined) {
    throw new Error("unsupported projection style purpose");
  }
  return variants[value]!;
}

/** Encodes one style setting scope as a Rust externally tagged enum.
 * @param value Browser scope descriptor.
 * @returns The Rust serde-compatible scope value.
 * @throws When the scope variant or identity is invalid.
 */
function styleScope(value: unknown): unknown {
  if (typeof value !== "object" || value === null)
    throw new Error("projection style scope is required");
  const scope = value as Record<string, unknown>;
  if (scope.kind === "user") return "User";
  if (scope.kind === "view" && typeof scope.viewId === "string") {
    return { View: { view_id: scope.viewId } };
  }
  if (scope.kind === "project" && typeof scope.projectId === "string") {
    return { Project: { project_id: scope.projectId } };
  }
  if (scope.kind === "workspace" && typeof scope.workspaceId === "string") {
    return { Workspace: { workspace_id: scope.workspaceId } };
  }
  throw new Error("unsupported projection style scope");
}

/** Encodes style operations using the pinned Rust protocol field names.
 * @param operation Protocol operation identifier.
 * @param payload Browser operation payload.
 * @returns Serde-compatible request body.
 * @throws When purpose or scope cannot be represented by the protocol.
 */
function projectionStyleRequest(operation: string, payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const value = payload as Record<string, unknown>;
  if (operation === "projection_style_schema") {
    return { projection_kind: value.projectionKind };
  }
  if (operation === "projection_style_resolve") {
    return {
      projection_kind: value.projectionKind,
      project_id: value.projectId,
      workspace_id: value.workspaceId,
      purpose: stylePurpose(value.purpose),
      view_id: value.viewId ?? null,
      explicit_personal: value.explicitPersonal ?? null,
    };
  }
  if (operation === "projection_style_setting") {
    return {
      scope: styleScope(value.scope),
      projection_kind: value.projectionKind,
      expected_revision: value.expectedRevision ?? null,
      style: value.style ?? null,
    };
  }
  return payload;
}

/** Encodes browser read requests using the pinned Rust serde representation.
 * @param operation Protocol operation identifier.
 * @param payload Browser request payload.
 * @returns The request body to serialize.
 * @throws When a typed request cannot be represented by the pinned contract.
 */
export function encodeRequest(operation: string, payload: unknown): unknown {
  if (operation === "inspect") return inspectionRequest(payload);
  if (operation === "list") return listRequest(payload);
  if (operation.startsWith("projection_style_")) return projectionStyleRequest(operation, payload);
  if (operation === "geo_feature_query") return geoFeatureQueryRequest(payload);
  if (operation === "cad_layer_change") return cadLayerChangeRequest(payload);
  if (operation === "cad_bounds_query") return cadBoundsQueryRequest(payload);
  if (operation === "qgis_layer_rename") return qgisLayerRenameRequest(payload);
  if (operation === "qgis_layer_order") return qgisLayerOrderRequest(payload);
  return payload;
}

/** Encodes a browser CAD bounds query using Rust serde field spelling.
 * @param payload Browser CAD bounds query.
 * @returns Serde-compatible request body.
 */
function cadBoundsQueryRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as Record<string, unknown>;
  const bounds = request.bounds;
  if (typeof bounds !== "object" || bounds === null) return payload;
  const envelope = bounds as Record<string, unknown>;
  return {
    ...request,
    bounds: {
      min_x: envelope.minX,
      min_y: envelope.minY,
      max_x: envelope.maxX,
      max_y: envelope.maxY,
    },
  };
}

/** Encodes a browser QGIS rename request with the Rust serde field spelling.
 * @param payload Browser QGIS rename request.
 * @returns Serde-compatible request body.
 */
function qgisLayerRenameRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as Record<string, unknown>;
  return {
    project: request.project,
    snapshot: request.snapshot,
    reference: request.reference,
    new_name: request.newName,
  };
}

/** Encodes a browser QGIS layer-order request using Rust serde field names.
 * @param payload Browser QGIS project layer-order request.
 * @returns Serde-compatible request body.
 */
function qgisLayerOrderRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as Record<string, unknown>;
  return {
    project: request.project,
    snapshot: request.snapshot,
    path: request.path,
    ordered_references: request.orderedReferences,
  };
}

/** Encodes layer-change requests using the pinned Rust field spelling.
 * @param payload Browser CAD layer-change request.
 * @returns Serde-compatible request body.
 */
function cadLayerChangeRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as Record<string, unknown>;
  return {
    project: request.project,
    snapshot: request.snapshot,
    entity: request.entity,
    destination_layer: request.destinationLayer,
  };
}

/** Encodes browser spatial bounds using the pinned Rust field spelling.
 * @param payload Browser geospatial query request.
 * @returns Serde-compatible query request.
 */
function geoFeatureQueryRequest(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null) return payload;
  const request = payload as Record<string, unknown>;
  const predicate = request.predicate;
  if (typeof predicate !== "object" || predicate === null) return payload;
  const value = predicate as Record<string, unknown>;
  if (value.kind !== "envelope_overlaps") return payload;
  const bounds = value.bounds;
  if (typeof bounds !== "object" || bounds === null) return payload;
  const envelope = bounds as Record<string, unknown>;
  return {
    ...request,
    predicate: {
      ...value,
      bounds: {
        min_x: envelope.minX,
        min_y: envelope.minY,
        max_x: envelope.maxX,
        max_y: envelope.maxY,
        crs: envelope.crs,
      },
    },
  };
}
