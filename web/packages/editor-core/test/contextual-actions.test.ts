/** @file Verifies stale contextual helper action requests are rejected. */

import { describe, expect, it } from "vitest";
import { ContextualActionRegistry } from "../src/contextual-actions";

describe("ContextualActionRegistry", () => {
  it("rejects unknown and stale requests while accepting current context", () => {
    const registry = new ContextualActionRegistry();
    registry.set([{ id: "inspect", label: "Inspect", mutating: false, contextVersion: 7 }]);

    expect(
      registry.validate({ invocationId: "one", actionId: "missing", contextVersion: 7 }),
    ).toMatchObject({ ok: false, invocationId: "one" });
    expect(
      registry.validate({ invocationId: "two", actionId: "inspect", contextVersion: 6 }),
    ).toMatchObject({ ok: false, contextVersion: 7 });
    expect(
      registry.validate({ invocationId: "three", actionId: "inspect", contextVersion: 7 }),
    ).toBeUndefined();
  });
});
