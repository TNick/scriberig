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

/** Encodes browser read requests using the pinned Rust serde representation.
 * @param operation Protocol operation identifier.
 * @param payload Browser request payload.
 * @returns The request body to serialize.
 * @throws When a typed request cannot be represented by the pinned contract.
 */
export function encodeRequest(operation: string, payload: unknown): unknown {
  if (operation === "inspect") return inspectionRequest(payload);
  if (operation === "list") return listRequest(payload);
  return payload;
}
