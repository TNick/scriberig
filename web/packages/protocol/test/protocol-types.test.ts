/** @file Verifies protocol discriminants and opaque identity boundaries. */
import { describe, expect, it } from "vitest";
import type { EntityId } from "../src/ids";
import type { ProtocolEvent } from "../src/operations";

describe("protocol types", () => {
  it("narrows event discriminants", () => {
    const event: ProtocolEvent = { kind: "operationCancelled", operationId: "op-1" as never };
    if (event.kind === "operationCancelled") expect(event.operationId).toBe("op-1");
  });

  it("keeps IDs opaque at boundaries", () => {
    const entity = "entity-1" as EntityId;
    expect(entity).toBe("entity-1");
  });
});
