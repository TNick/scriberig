/** @file Verifies client operation mapping and event subscription wiring. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";
describe("ScriberigClient", () => {
  it("maps operations through injected transport", async () => {
    const calls: string[] = [];
    const transport = {
      request: async <T>(operation: string): Promise<T> => {
        calls.push(operation);
        return [] as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    await client.find({ snapshot: { kind: "current" }, text: "hello" });
    expect(calls).toEqual(["find"]);
  });

  it("unwraps the typed project response result", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return { result: { projects: [] } } as T;
      },
      subscribe: () => () => undefined,
    };

    const projects = await new ScriberigClient(transport).project<{ projects: unknown[] }>(
      "list",
      {},
    );

    expect(projects).toEqual({ projects: [] });
    expect(calls).toEqual([["project", { operation: "list", payload: {} }]]);
  });

  it("reads canonical migration detail through the public transport", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          migrationId: "migration-1",
          base: "snapshot-1",
          result: "snapshot-2",
          source: "canonical",
          changes: [],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const detail = await new ScriberigClient(transport).migrationDetail("migration-1");
    expect(detail.migrationId).toBe("migration-1");
    expect(calls).toEqual([["migration", { migrationId: "migration-1" }]]);
  });

  it("executes a public application command without owning semantic mutation logic", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return { operationId: "operation-1", kind: "apply", status: "queued" } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    await client.command("apply", { selection: "entity-1" });

    expect(calls).toEqual([["command", { kind: "apply", payload: { selection: "entity-1" } }]]);
  });

  it("exposes the public application operation families", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return { operationId: "operation-1", kind: "test", status: "queued" } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    await client.branch({ branchId: "branch-1" });
    await client.migrate({ proposalId: "proposal-1" });
    await client.validate({ proposalId: "proposal-1" });
    await client.render({ projectionId: "projection-1" });
    await client.session({ sessionId: "session-1" });
    await client.agent({ prompt: "summarize" });

    expect(calls).toEqual([
      ["command", { kind: "branch", payload: { branchId: "branch-1" } }],
      ["command", { kind: "migrate", payload: { proposalId: "proposal-1" } }],
      ["command", { kind: "validate", payload: { proposalId: "proposal-1" } }],
      ["command", { kind: "render", payload: { projectionId: "projection-1" } }],
      ["command", { kind: "session", payload: { sessionId: "session-1" } }],
      ["command", { kind: "agent", payload: { prompt: "summarize" } }],
    ]);
  });

  it("exposes provider-neutral agent and lifecycle methods", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return (
          operation === "agent_start"
            ? { operationId: "op-1", sessionId: "session-1", branchId: "branch-1" }
            : operation === "session"
              ? {
                  session: { id: "session-1" },
                  branches: [
                    {
                      id: "branch-1",
                      title: "Root",
                      status: "Active",
                      children: [],
                      turns: [],
                    },
                  ],
                }
              : { operationId: "op-1", status: "cancelled", kind: "agent" }
        ) as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    await client.startAgent({ sessionId: "session-1" as never, prompt: "work" });
    await client.listBranchChildren("session-1" as never, "branch-1" as never);
    await client.cancelOperation("op-1");

    expect(calls).toEqual([
      ["agent_start", { session: { id: "session-1" }, prompt: "work" }],
      ["session", { session: { id: "session-1" } }],
      ["cancel", { operation: { id: "op-1" } }],
    ]);
  });

  it("normalizes the Rust protocol operation envelope at the web boundary", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          reference: { id: "operation-2" },
          kind: "Apply",
          status: "Running",
          progress: { percent: 25, message: "working" },
        }) as T,
      subscribe: () => () => undefined,
    };

    const operation = await new ScriberigClient(transport).command("apply", {});

    expect(operation).toMatchObject({
      operationId: "operation-2",
      kind: "apply",
      status: "running",
      progress: 0.25,
    });
  });

  it("unwraps correlated Rust response envelopes", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        async () =>
          new Response(
            JSON.stringify({
              version: { major: 1, minor: 0 },
              correlation_id: "corr-1",
              response: {
                Capabilities: {
                  protocol: { major: 1, minor: 0 },
                  capabilities: [],
                  features: [],
                },
              },
            }),
            { status: 200 },
          ),
      ),
    );
    const capabilities = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).capabilities();
    expect(capabilities).toMatchObject({ capabilities: [], features: [] });
    vi.unstubAllGlobals();
  });

  it("marks protocol calls with the typed protocol header", async () => {
    const fetch = vi.fn(async () => new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetch);
    await new HttpTransport("http://test/api").request("agent_start", {});
    expect(fetch).toHaveBeenCalledWith(
      "http://test/api/protocol/agent_start",
      expect.objectContaining({
        headers: expect.objectContaining({ "x-scriberig-protocol": "1" }),
      }),
    );
    vi.unstubAllGlobals();
  });
});
