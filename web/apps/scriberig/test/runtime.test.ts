/** @file Verifies browser runtime bootstrap and connection failure behavior. */

import { afterEach, describe, expect, it, vi } from "vitest";
import { createRuntime } from "../src/runtime";

const ROOT = "ent_01010101010101010101010101010101" as never;
const SNAPSHOT = "snp_01010101010101010101010101010101" as never;

describe("createRuntime", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("bootstraps a connected root from public capabilities and inspect calls", async () => {
    const fetch = vi.fn(async (url: string) => {
      if (url.endsWith("/capabilities")) {
        return new Response(
          JSON.stringify({
            protocolMajor: 1,
            features: ["inspect", "list"],
            capabilities: [],
            root: { entityId: ROOT, snapshotId: SNAPSHOT },
          }),
          { status: 200 },
        );
      }
      return new Response(
        JSON.stringify({
          summary: {
            ref: { entityId: ROOT, snapshotId: SNAPSHOT },
            typeId: "scriberig.demo.entity",
            label: "Development root",
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const runtime = createRuntime("http://test/api");
    await runtime.bootstrap();

    expect(runtime.connection).toBe("connected");
    expect(runtime.tree.get("root")?.entityId).toBe(ROOT);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it("reports disconnected when bootstrap transport fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    const runtime = createRuntime("http://test/api");
    await expect(runtime.bootstrap()).rejects.toThrow("semantic transport request failed");
    expect(runtime.connection).toBe("disconnected");
  });
});
