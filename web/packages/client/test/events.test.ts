/** @file Verifies protocol event projection and resumable HTTP subscriptions. */
import { describe, expect, it, vi } from "vitest";
import type { ProtocolEvent } from "@scriberig/protocol";
import { HttpTransport, ScriberigClient, normalizeProtocolEvent } from "../src";
describe("protocol event delivery", () => {
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

  it("projects resumable protocol events into agent events", () => {
    let listener: ((event: ProtocolEvent) => void) | undefined;
    const transport = {
      request: async <T>() => [] as T,
      subscribe: (callback: (event: ProtocolEvent) => void) => {
        listener = callback;
        return () => undefined;
      },
    };
    const received: unknown[] = [];
    new ScriberigClient(transport).subscribeAgentEvents((event) => received.push(event));
    listener?.({
      kind: "agentOutput",
      sequence: 4,
      operationId: "op-1" as never,
      text: "committed answer",
    });
    expect(received).toEqual([
      {
        kind: "agentOutputCommitted",
        sequence: 4,
        operationId: "op-1",
      },
    ]);
  });

  it("delivers resumable SSE events through the public HTTP transport", async () => {
    const event: ProtocolEvent = { kind: "snapshotAdvanced", snapshotId: "s1" as never };
    const fetch = vi.fn(
      async () =>
        new Response(
          `id: 7\nevent: protocol\ndata: ${JSON.stringify({
            kind: "SnapshotChanged",
            snapshot: { id: "s1" },
          })}\n\n`,
          {
            status: 200,
            headers: { "content-type": "text/event-stream" },
          },
        ),
    );
    vi.stubGlobal("fetch", fetch);
    const transport = new HttpTransport("http://test/api");
    const received: ProtocolEvent[] = [];

    const subscription = transport.subscribe((value) => received.push(value));
    await vi.waitFor(() => expect(received).toEqual([event]));

    subscription();
    expect(fetch).toHaveBeenCalledWith(
      "http://test/api/protocol/events",
      expect.objectContaining({ headers: expect.any(Object) }),
    );
    vi.unstubAllGlobals();
  });

  it("reconnects the SSE stream until it is closed", async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(
      async () =>
        new Response(": heartbeat\n\n", {
          status: 200,
          headers: { "content-type": "text/event-stream" },
        }),
    );
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

  it("reports event transport disconnect and reconnect transitions", async () => {
    vi.useFakeTimers();
    let attempt = 0;
    const fetch = vi.fn(async () => {
      attempt += 1;
      if (attempt === 1) throw new TypeError("offline");
      return new Response(": heartbeat\n\n", {
        status: 200,
        headers: { "content-type": "text/event-stream" },
      });
    });
    vi.stubGlobal("fetch", fetch);
    const states: string[] = [];
    const transport = new HttpTransport("http://test/api");
    const unsubscribe = transport.subscribe(
      () => undefined,
      undefined,
      (state) => states.push(state),
    );

    await vi.waitFor(() => expect(states).toEqual(["disconnected"]));
    await vi.advanceTimersByTimeAsync(1_000);
    await vi.waitFor(() => expect(states).toEqual(["disconnected", "connected"]));

    unsubscribe();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("aborts an active SSE request when the subscription closes", async () => {
    let requestSignal: AbortSignal | undefined;
    const fetch = vi.fn(
      (_input: string, init?: RequestInit) =>
        new Promise<Response>((_resolve, _reject) => {
          requestSignal = init?.signal ?? undefined;
        }),
    );
    vi.stubGlobal("fetch", fetch);
    const transport = new HttpTransport("http://test/api");
    const unsubscribe = transport.subscribe(() => undefined);
    await vi.waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    unsubscribe();
    expect(requestSignal?.aborted).toBe(true);
    vi.unstubAllGlobals();
  });

  it("normalizes Rust event envelopes at the HTTP boundary", () => {
    expect(
      normalizeProtocolEvent({
        kind: "SnapshotChanged",
        snapshot: { id: "s2" },
        metadata: [],
      }),
    ).toEqual({ kind: "snapshotAdvanced", snapshotId: "s2" });
    expect(
      normalizeProtocolEvent({
        kind: "OperationProgressed",
        operation: { id: "op-1" },
        metadata: [["percent", "25"]],
      }),
    ).toEqual({ kind: "operationProgress", operationId: "op-1", progress: 0.25 });
    expect(
      normalizeProtocolEvent({
        kind: "SessionChanged",
        entity: { id: "session-1" },
        metadata: [],
      }),
    ).toEqual({ kind: "sessionChanged", sessionId: "session-1" });
  });
});
