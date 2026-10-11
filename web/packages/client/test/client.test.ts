/** @file Verifies client operation mapping and event subscription wiring. */
import { describe, expect, it } from "vitest";
import type { ProtocolEvent } from "@scriberig/protocol";
import { ScriberigClient } from "../src";
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
});
