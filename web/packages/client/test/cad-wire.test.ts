/** @file Verifies CAD request wire encoding and response normalization. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";

describe("CAD wire contract", () => {
  it("encodes CAD layer-change request fields using Rust snake case", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        entity: "entity-3",
        destination_layer: "layer-4",
      });
      return new Response(
        JSON.stringify({
          response: {
            CadLayerChange: {
              candidate: "candidate-5",
              project: "project-1",
              base_snapshot: "snapshot-2",
              path: "drawing.dxf",
              validation: { state: "passed" },
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);
    const candidate = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).proposeCadLayerChange({
      project: "project-1",
      snapshot: "snapshot-2",
      entity: "entity-3",
      destinationLayer: "layer-4",
    });
    expect(candidate).toMatchObject({
      candidateId: "candidate-5",
      validation: { status: "passed" },
    });
    vi.unstubAllGlobals();
  });
});
