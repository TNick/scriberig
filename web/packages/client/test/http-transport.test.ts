/** @file Verifies HTTP transport route negotiation and response decoding. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";

describe("HttpTransport", () => {
  it("uses the legacy capabilities response for application bootstrap", async () => {
    const request = vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            protocolMajor: 1,
            capabilities: [],
            features: ["inspect", "list"],
            root: { entityId: "entity-1", snapshotId: "snapshot-1" },
          }),
          { status: 200 },
        ),
    );
    vi.stubGlobal("fetch", request);
    const capabilities = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).capabilities();
    expect(capabilities).toMatchObject({
      protocolMajor: 1,
      capabilities: [],
      features: ["inspect", "list"],
      root: { entityId: "entity-1", snapshotId: "snapshot-1" },
    });
    expect(request).toHaveBeenCalledWith(
      "http://test/api/protocol/capabilities",
      expect.objectContaining({
        headers: expect.objectContaining({
          "content-type": "application/json",
          accept: "application/json",
        }),
      }),
    );
    vi.unstubAllGlobals();
  });

  it("uses the typed protocol envelope for protocol-only operations", async () => {
    const request = vi.fn(async (_input: string, init: RequestInit) => {
      expect(new Headers(init.headers).get("accept")).toBe(
        "application/vnd.scriberig.protocol+json",
      );
      return new Response(
        JSON.stringify({
          version: { major: 1, minor: 0 },
          correlation_id: "corr-1",
          response: { ModelLibrary: { models: [] } },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", request);
    const response = await new HttpTransport("http://test/api").request<{
      models: unknown[];
    }>("model_library", {});
    expect(response).toEqual({ models: [] });
    expect(request).toHaveBeenCalledWith(
      "http://test/api/protocol/model_library",
      expect.objectContaining({
        headers: expect.objectContaining({
          "content-type": "application/json",
          accept: "application/vnd.scriberig.protocol+json",
        }),
      }),
    );
    vi.unstubAllGlobals();
  });
});
