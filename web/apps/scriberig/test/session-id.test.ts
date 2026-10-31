/** @file Verifies browser session identifiers satisfy the Rust protocol format. */

import { describe, expect, it } from "vitest";
import { getOrCreateSessionId } from "../src/session-id";

describe("browser session identity", () => {
  it("replaces legacy UUID values with canonical entity identifiers", () => {
    const values = new Map([["scriberig.agent.session", "0eac8a0f-df40-46c6-9d5a-6e48f56f3b37"]]);
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    };

    const sessionId = getOrCreateSessionId(storage);

    expect(sessionId).toMatch(/^ent_[a-f0-9]{32}$/);
    expect(values.get("scriberig.agent.session")).toBe(sessionId);
  });

  it("reuses a previously stored canonical session identifier", () => {
    const stored = "ent_01010101010101010101010101010101";
    const storage = {
      getItem: () => stored,
      setItem: () => {
        throw new Error("valid stored id should not be rewritten");
      },
    };

    expect(getOrCreateSessionId(storage)).toBe(stored);
  });
});
