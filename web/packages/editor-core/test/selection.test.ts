/** @file Tests semantic selection and command discovery. */
import { describe, expect, it } from "vitest";
import type { EntityId, RevisionId, SnapshotId } from "@scriberig/protocol";
import { SelectionController, sameSelection } from "../src/selection";
import { discoverCommands } from "../src/commands";

describe("editor core primitives", () => {
  it("retains semantic identity while projection location changes", () => {
    const controller = new SelectionController();
    const selection = {
      entityId: "ent_a" as EntityId,
      snapshotId: "snp_a" as SnapshotId,
      revisionId: "rev_a" as RevisionId,
    };
    controller.set(selection);
    controller.updateProjection({ key: "file", location: "line-1" });
    expect(sameSelection(controller.get(), selection)).toBe(true);
  });
  it("filters commands by capability and applicability", () => {
    const commands = [
      {
        id: "read",
        title: "Read",
        mutating: false,
        requiredCapability: "inspect",
        execute: "inspect",
      },
    ];
    expect(discoverCommands(commands, undefined, new Set(["inspect"]))).toHaveLength(1);
    expect(discoverCommands(commands, undefined, new Set())).toHaveLength(0);
  });
});
