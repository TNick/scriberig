/** @file Verifies client operation mapping and event subscription wiring. */
import { describe, expect, it, vi } from "vitest";
import type { ProtocolEvent } from "@scriberig/protocol";
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
  it("wires typed event subscriptions and close state", () => {
    let listener: ((event: ProtocolEvent) => void) | undefined;
    const transport = {
      request: async <T>() => [] as T,
      subscribe: (callback: (event: ProtocolEvent) => void) => {
        listener = callback;
        return () => {
          listener = undefined;
        };
      },
    };
    const client = new ScriberigClient(transport);
    const subscription = client.events(() => undefined);
    expect(subscription.state).toBe("connected");
    subscription.close();
    expect(subscription.state).toBe("disconnected");
  });

  it("delivers buffered protocol events through the public HTTP transport", async () => {
    const event: ProtocolEvent = { kind: "snapshotAdvanced", snapshotId: "s1" as never };
    const fetch = vi.fn(async () => new Response(JSON.stringify([event]), { status: 200 }));
    vi.stubGlobal("fetch", fetch);
    const transport = new HttpTransport("http://test/api");
    const received: ProtocolEvent[] = [];

    const subscription = transport.subscribe((value) => received.push(value));
    await vi.waitFor(() => expect(received).toEqual([event]));

    subscription();
    expect(fetch).toHaveBeenCalledWith(
      "http://test/api/protocol/subscribe",
      expect.objectContaining({ method: "POST" }),
    );
    vi.unstubAllGlobals();
  });

  it("continues polling the public event stream until it is closed", async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(async () => new Response("[]", { status: 200 }));
    vi.stubGlobal("fetch", fetch);
    const transport = new HttpTransport("http://test/api");
    const unsubscribe = transport.subscribe(() => undefined);

    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    await vi.advanceTimersByTimeAsync(1_000);
    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(2));

    unsubscribe();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(fetch).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });
});
