/** @file Verifies bounded artifact inspection paging and continuation behavior. */
import { describe, expect, it, vi } from "vitest";
import { ArtifactPageController } from "../src";

describe("ArtifactPageController", () => {
  it("requests bounded pages and follows the server continuation", async () => {
    const requests: unknown[] = [];
    const client = {
      inspectArtifacts: vi.fn(async (request: unknown) => {
        requests.push(request);
        const offset = (request as { offset: number }).offset;
        return {
          project: "project-1",
          artifacts: [],
          total: 3,
          ...(offset < 2 ? { nextOffset: offset + 2 } : {}),
        };
      }),
    };
    const controller = new ArtifactPageController(client as never, "project-1", 2);
    await controller.read();
    await controller.next();
    expect(await controller.next()).toBeUndefined();
    expect(requests).toEqual([
      { project: "project-1", offset: 0, limit: 2 },
      { project: "project-1", offset: 2, limit: 2 },
    ]);
    expect(controller.currentPage()?.total).toBe(3);
    expect(controller.canPrevious()).toBe(true);
    await controller.previous();
    expect(controller.currentPage()?.nextOffset).toBe(2);
    expect(vi.mocked(client.inspectArtifacts)).toHaveBeenCalledTimes(3);
  });

  it("rejects a page size above the shared application maximum", () => {
    expect(
      () => new ArtifactPageController({ inspectArtifacts: vi.fn() } as never, "p", 101),
    ).toThrow(RangeError);
  });

  it("rejects invalid offsets before calling the client", async () => {
    const client = { inspectArtifacts: vi.fn() };
    const controller = new ArtifactPageController(client as never, "p");
    await expect(controller.read(-1)).rejects.toThrow(RangeError);
    expect(client.inspectArtifacts).not.toHaveBeenCalled();
  });
});
