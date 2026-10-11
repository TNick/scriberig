/** @file Verifies protocol discriminants and opaque identity boundaries. */
import { describe, expect, it } from "vitest";
import type { EntityId } from "../src/ids";
import type { ProtocolEvent } from "../src/operations";
import type { ConflictSummary, PromotionResult, TrustEvent, ValidationSummary } from "../src/trust";

describe("protocol types", () => {
  it("narrows event discriminants", () => {
    const event: ProtocolEvent = { kind: "operationCancelled", operationId: "op-1" as never };
    if (event.kind === "operationCancelled") expect(event.operationId).toBe("op-1");
  });

  it("keeps IDs opaque at boundaries", () => {
    const entity = "entity-1" as EntityId;
    expect(entity).toBe("entity-1");
  });

  it("exposes structured trust states and event handles", () => {
    const validation: ValidationSummary = {
      status: "failed",
      diagnostics: [{ code: "schema", message: "invalid" }],
    };
    const conflict: ConflictSummary = { handle: "conflict-1" };
    const promotion: PromotionResult = { state: "stale", reason: "changed" };
    const event: TrustEvent = {
      type: "snapshot_promoted",
      selection: {
        entityId: "entity-1" as never,
        snapshotId: "snapshot-1" as never,
      },
      result: promotion,
    };
    expect(validation.status).toBe("failed");
    expect(conflict.handle).toBe("conflict-1");
    expect(event.type).toBe("snapshot_promoted");
  });
});
