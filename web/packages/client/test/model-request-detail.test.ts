/** @file Verifies retrieval of captured model request detail. */
import { describe, expect, it } from "vitest";
import type { BranchId, SessionId } from "@scriberig/protocol";
import { ScriberigClient } from "../src";

describe("model request detail", () => {
  it("loads a captured request projection by invocation identity", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          invocation: "invocation-1",
          model: "model-a",
          provider_id: "provider-a",
          messages: [{ role: "user", content: "safe prompt" }],
          tools: [],
          truncated: false,
        } as T;
      },
      subscribe: () => () => undefined,
    };

    const detail = await new ScriberigClient(transport).getModelRequestDetail(
      "session-1" as SessionId,
      "branch-1" as BranchId,
      "invocation-1",
    );

    expect(detail.messages[0]?.content).toBe("safe prompt");
    expect(detail.providerId).toBe("provider-a");
    expect(calls).toEqual([
      [
        "model_request_detail",
        {
          session: { id: "session-1" },
          branch: { id: "branch-1" },
          invocation: "invocation-1",
        },
      ],
    ]);
  });

  it("loads bounded tool-result detail by its durable identity", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          result: "result-1",
          detail: { summary: "safe result" },
          truncated: false,
        } as T;
      },
      subscribe: () => () => undefined,
    };

    const detail = await new ScriberigClient(transport).getToolResultDetail(
      "session-1" as SessionId,
      "branch-1" as BranchId,
      "result-1",
    );

    expect(detail.detail.summary).toBe("safe result");
    expect(calls[0]?.[0]).toBe("tool_result_detail");
  });
});
