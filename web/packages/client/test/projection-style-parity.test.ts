/** @file Verifies web style normalization against the shared Rust/TUI fixture. */

import { describe, expect, it, vi } from "vitest";
import styleResolution from "../../../../tests/fixtures/projection-style-rust.json";
import { HttpTransport, ScriberigClient } from "../src";

describe("projection style cross-surface fixture", () => {
  it("normalizes the same effective Rust style and fidelity consumed by TUI", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response(JSON.stringify({ response: { ProjectionStyleResolve: styleResolution } }), {
            status: 200,
          }),
      ),
    );
    const client = new ScriberigClient(new HttpTransport("http://test/api"));

    const resolved = await client.resolveProjectionStyle({
      projectionKind: "rust",
      projectId: "fixture-project",
      workspaceId: "fixture-project",
      purpose: "interactiveView",
      viewId: "source-view",
    });

    expect(resolved.value).toEqual(styleResolution.value);
    expect(resolved.applied).toEqual([
      expect.objectContaining({
        kind: "user-preference",
        identity: "fixture-user",
        generation: 2,
      }),
    ]);
    expect(resolved.fidelity).toEqual({
      fidelity: "losslessEditable",
      omittedCategories: [],
    });
    vi.unstubAllGlobals();
  });
});
