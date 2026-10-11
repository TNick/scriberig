/** @file Verifies mutation tests use public command descriptors. */
import { expect, it } from "vitest";
import { discoverCommands, type CommandDescriptor } from "@scriberig/editor-core";
it("discovers a public mutating command", () => {
  const c: CommandDescriptor = { id: "apply", title: "Apply", mutating: true, execute: "apply" };
  expect(discoverCommands([c], undefined, new Set())).toHaveLength(1);
});
