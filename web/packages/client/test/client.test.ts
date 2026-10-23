/** @file Verifies client operation mapping and event subscription wiring. */
import { describe, expect, it, vi } from "vitest";
import type { BranchId, ProtocolEvent, SessionId } from "@scriberig/protocol";
import { HttpTransport, ScriberigClient, normalizeProtocolEvent } from "../src";
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
    const request = vi.fn(async (_input: string, init: RequestInit) => {
      expect(new Headers(init.headers).get("accept")).toBe(
        "application/vnd.scriberig.protocol+json",
      );
      return new Response(
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
      );
    });
    vi.stubGlobal("fetch", request);
    const capabilities = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).capabilities();
    expect(capabilities).toMatchObject({ capabilities: [], features: [] });
    expect(request).toHaveBeenCalledWith(
      "http://test/api/protocol/capabilities",
      expect.objectContaining({
        headers: expect.objectContaining({
          "content-type": "application/json",
          accept: "application/vnd.scriberig.protocol+json",
        }),
      }),
    );
    vi.unstubAllGlobals();
  });

  it("normalizes durable agent results from operation reads", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          reference: { id: "op-agent" },
          kind: "Agent",
          status: "Completed",
          progress: null,
          result_snapshot: null,
          result_handles: [],
          agent_result: {
            session: { id: "session-1" },
            root_branch: { id: "branch-1" },
            final_output: "done",
            branch_result: "result-1",
            produced_changes: [{ id: "ent-1" }],
            validation_results: [],
            unresolved_questions: [],
            candidate_changes: [
              {
                migration: "migration-1",
                entity: { id: "ent-1" },
                candidate_snapshot: { id: "snapshot-2" },
                validation: ["validation-1"],
              },
            ],
          },
          error: null,
        }) as T,
      subscribe: () => () => undefined,
    };
    const result = await new ScriberigClient(transport).getAgentResult("op-agent");
    expect(result).toMatchObject({
      sessionId: "session-1",
      rootBranchId: "branch-1",
      branchResultId: "result-1",
      finalOutput: "done",
      handles: ["ent-1"],
      candidateChanges: [
        {
          migrationId: "migration-1",
          entityId: "ent-1",
          candidateSnapshotId: "snapshot-2",
          validationRefs: ["validation-1"],
        },
      ],
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
