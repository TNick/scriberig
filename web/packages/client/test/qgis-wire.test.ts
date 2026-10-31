/** @file Verifies QGIS protocol wire encoding and response normalization. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";
describe("QGIS protocol wire contracts", () => {
  it("encodes complete project layer order and normalizes its candidate handle", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        path: "project.qgs",
        ordered_references: ["reference-2", "reference-1"],
      });
      return new Response(
        JSON.stringify({
          response: {
            QgisLayerOrder: {
              candidate: "candidate-4",
              project: "project-1",
              base_snapshot: "snapshot-2",
              path: "project.qgs",
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const candidate = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).proposeQgisLayerOrder({
      project: "project-1",
      snapshot: "snapshot-2",
      path: "project.qgs",
      orderedReferences: ["reference-2", "reference-1"],
    });

    expect(candidate).toEqual({
      candidate: "candidate-4",
      project: "project-1",
      baseSnapshot: "snapshot-2",
      path: "project.qgs",
    });
    expect(fetch).toHaveBeenCalledOnce();
    vi.unstubAllGlobals();
  });

  it("encodes QGIS rename names and normalizes the shared candidate handle", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        reference: "reference-3",
        new_name: "Lots",
      });
      return new Response(
        JSON.stringify({
          response: {
            QgisLayerRename: {
              candidate: "candidate-4",
              project: "project-1",
              base_snapshot: "snapshot-2",
              reference: "reference-3",
              path: "project.qgs",
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const candidate = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).proposeQgisLayerRename({
      project: "project-1",
      snapshot: "snapshot-2",
      reference: "reference-3",
      newName: "Lots",
    });

    expect(candidate).toEqual({
      candidate: "candidate-4",
      project: "project-1",
      baseSnapshot: "snapshot-2",
      reference: "reference-3",
      path: "project.qgs",
    });
    expect(fetch).toHaveBeenCalledOnce();
    vi.unstubAllGlobals();
  });
});
