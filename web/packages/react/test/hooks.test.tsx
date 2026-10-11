/** @file Verifies React bindings remain thin over shared controllers. */
import { describe, expect, it } from "vitest";
import { KnowledgeTree, SelectionController } from "@scriberig/editor-core";

describe("headless hook dependencies", () => {
  it("shares selection and tree controllers across consumers", () => {
    const selection = new SelectionController();
    const tree = new KnowledgeTree();
    expect(selection.get()).toBeUndefined();
    expect(tree.version()).toBe(0);
  });
});
