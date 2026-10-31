/** @file Verifies projection-style presentation uses the shared client contract. */
import { describe, expect, it, vi } from "vitest";
import { ProjectionStyleController } from "../src";

describe("ProjectionStyleController", () => {
  it("forwards schema, resolution, and setting operations without local style semantics", async () => {
    const schema = {
      reference: { adapterId: "ecma", version: "1" },
      projectionKinds: ["javascript"],
      properties: [],
    };
    const resolution = {
      purpose: "interactiveView" as const,
      reference: { kind: "inline" as const },
      value: [123, 125],
      applied: [],
      fidelity: { fidelity: "losslessEditable" as const, omittedCategories: [] },
    };
    const setting = { style: [123, 125], revision: 3 };
    const client = {
      projectionStyleSchema: vi.fn(async () => schema),
      resolveProjectionStyle: vi.fn(async () => resolution),
      setProjectionStyle: vi.fn(async () => setting),
    };
    const controller = new ProjectionStyleController(client as never);
    const signal = new AbortController().signal;
    const request = {
      projectionKind: "javascript",
      projectId: "project-1",
      workspaceId: "workspace-1",
      purpose: "interactiveView" as const,
      viewId: "view-1",
    };
    const write = {
      projectionKind: "javascript",
      scope: { kind: "view" as const, viewId: "view-1" },
      style: [123, 125],
    };

    await expect(controller.schema("javascript", signal)).resolves.toEqual(schema);
    await expect(controller.resolve(request, signal)).resolves.toEqual(resolution);
    await expect(controller.set(write, signal)).resolves.toEqual(setting);
    expect(client.projectionStyleSchema).toHaveBeenCalledWith("javascript", signal);
    expect(client.resolveProjectionStyle).toHaveBeenCalledWith(request, signal);
    expect(client.setProjectionStyle).toHaveBeenCalledWith(write, signal);
  });
});
