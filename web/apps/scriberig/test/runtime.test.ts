/** @file Verifies browser runtime bootstrap and connection failure behavior. */

import { afterEach, describe, expect, it, vi } from "vitest";
import { capabilityAvailable, createRuntime } from "../src/runtime";

const ROOT = "ent_01010101010101010101010101010101" as never;
const SNAPSHOT = "snp_01010101010101010101010101010101" as never;

describe("createRuntime", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("projects capability and feature availability without private fallbacks", () => {
    const capabilities = {
      protocolMajor: 1 as const,
      features: ["agent_start"],
      capabilities: [{ id: "operation_cancel", description: "Cancel", available: true }],
    };
    expect(capabilityAvailable(capabilities, "agent_start")).toBe(true);
    expect(capabilityAvailable(capabilities, "operation_cancel")).toBe(true);
    expect(capabilityAvailable(capabilities, "agent_continue")).toBe(false);
    expect(capabilityAvailable(undefined, "agent_start")).toBe(false);
  });

  it("bootstraps a connected root from public capabilities and inspect calls", async () => {
    const fetch = vi.fn(async (url: string) => {
      if (url.includes("/events")) {
        return new Response(
          [
            { kind: "snapshotAdvanced", snapshotId: "s2" },
            {
              kind: "AgentActivity",
              sequence: 5,
              operation: { id: "op-1" },
              metadata: [
                ["branch", "branch-1"],
                ["activity_kind", "toolInvocation"],
                ["state", "running"],
                ["summary", "Tool: search"],
              ],
            },
            {
              kind: "AgentOutputProduced",
              sequence: 6,
              operation: { id: "op-1" },
              metadata: [["output", "committed answer"]],
            },
          ]
            .map((event) => `data: ${JSON.stringify(event)}\n\n`)
            .join(""),
          {
            headers: { "content-type": "text/event-stream" },
            status: 200,
          },
        );
      }
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
    let liveActivityObserved = false;
    runtime.model.subscribeAgent((state) => {
      liveActivityObserved ||=
        state.transientActivities["branch-1"]?.some((item) => item.summary === "Tool: search") ===
        true;
    });
    await runtime.bootstrap();

    expect(runtime.connection).toBe("connected");
    expect(runtime.tree.get("root")?.entityId).toBe(ROOT);
    expect(runtime.commands.map((command) => command.id)).toEqual(["inspect", "list"]);
    await vi.waitFor(() => expect(runtime.model.getState().snapshots.current).toBe("s2"));
    await vi.waitFor(() => expect(liveActivityObserved).toBe(true));
    expect(runtime.model.getAgentState().transientActivities).toEqual({});
    expect(fetch).toHaveBeenCalledTimes(3);
  });

  it("reports disconnected when bootstrap transport fails", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    const runtime = createRuntime("http://test/api");
    await expect(runtime.bootstrap()).rejects.toThrow("semantic transport request failed");
    expect(runtime.connection).toBe("disconnected");
  });
});
