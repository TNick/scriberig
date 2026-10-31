/** @file Verifies resolved projection styles remain attached to semantic view state. */

import { describe, expect, it } from "vitest";
import type { ProjectionSession } from "../src/projection-session";
import { withResolvedProjectionStyle } from "../src/projection-session";

const session: ProjectionSession = {
  selection: { entityId: "entity-1" as never, snapshotId: "snapshot-1" as never },
  projectionId: "source",
  baseSnapshot: "snapshot-1",
  stale: false,
  conflict: false,
};

describe("withResolvedProjectionStyle", () => {
  it("retains exact style evidence without changing semantic session state", () => {
    const style = {
      purpose: "interactiveView" as const,
      reference: { kind: "inline" as const },
      value: [105, 110, 100, 101, 110, 116, 61, 116, 97, 98, 115],
      applied: [{ kind: "user-preference", identity: "user-1", value: [1, 2] }],
      fidelity: { fidelity: "losslessEditable" as const, omittedCategories: [] },
    };

    expect(withResolvedProjectionStyle(session, style)).toEqual({
      ...session,
      resolvedStyle: style,
    });
    expect(session).not.toHaveProperty("resolvedStyle");
  });

  it("rejects a materialization style in an interactive projection session", () => {
    expect(() =>
      withResolvedProjectionStyle(session, {
        purpose: "sharedMaterialization",
        reference: { kind: "inline" },
        value: [],
        applied: [],
        fidelity: { fidelity: "losslessEditable", omittedCategories: [] },
      }),
    ).toThrow("interactive view style");
  });
});
