/** @file Verifies mutation tests use public command descriptors. */
import { expect, it } from "vitest";
import { discoverCommands, type CommandDescriptor } from "@scriberig/editor-core";
import { ScriberigClient } from "@scriberig/client";
it("discovers a public mutating command", () => {
  const c: CommandDescriptor = { id: "apply", title: "Apply", mutating: true, execute: "apply" };
  expect(discoverCommands([c], undefined, new Set())).toHaveLength(1);
});

it("routes migration and validation through public operations", async () => {
  const calls: Array<[string, unknown]> = [];
  async function request<T>(operation: string, payload: unknown): Promise<T> {
    calls.push([operation, payload]);
    return { operationId: "operation-1", kind: operation, status: "queued" } as T;
  }
  const transport = {
    request,
    subscribe: () => () => undefined,
  };
  const client = new ScriberigClient(transport);

  await client.migrate({ proposalId: "proposal-1" });
  await client.validate({ proposalId: "proposal-1" });

  expect(calls).toEqual([
    ["command", { kind: "migrate", payload: { proposalId: "proposal-1" } }],
    ["command", { kind: "validate", payload: { proposalId: "proposal-1" } }],
  ]);
});
