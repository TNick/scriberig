/** @file Verifies authoritative session and branch projections. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
import { normalizeModelLibrary } from "../src/model-views.js";
import { normalizeSession } from "../src/session-views.js";

describe("session client projections", () => {
  it("sends typed plan semantic mutations through the shared operation", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          id: "plan-1",
          revision: 3,
          goal: "Ship feature",
          status: "Running",
          root_branch: "branch-1",
          nodes: [],
          dependencies: [],
          root_nodes: [],
          status_counts: {},
          gates: [],
          evidence: [],
          summaries: [],
          history: [],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const plan = await client.mutatePlanSemantics("session-1" as never, "plan-1", 2, {
      UpdateSummary: {
        node: "node-1",
        text: "Validation passed",
        sources: [{ entity: "validation-1", revision: "revision-1" }],
      },
    });

    expect(plan.id).toBe("plan-1");
    expect(plan.statusCounts).toEqual({
      planned: 0,
      ready: 0,
      running: 0,
      completed: 0,
      failed: 0,
      abandoned: 0,
    });
    expect(requests).toEqual([
      {
        operation: "plan_semantic_mutation",
        payload: {
          session: { id: "session-1" },
          plan: "plan-1",
          expected_revision: 2,
          mutation: {
            UpdateSummary: {
              node: "node-1",
              text: "Validation passed",
              sources: [{ entity: "validation-1", revision: "revision-1" }],
            },
          },
        },
      },
    ]);
    await client.retryPlanNode("session-1" as never, "plan-1", "node-1", 3);
    expect(requests[1]).toEqual({
      operation: "plan_retry",
      payload: {
        session: { id: "session-1" },
        plan: "plan-1",
        node: "node-1",
        expected_revision: 3,
      },
    });
  });

  it("reads and revision-checks shared presentation preferences", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        return {
          revision: 3,
          preferences: {
            reasoning: "shown",
            tool_invocation: "detailed",
            tool_output: "full",
            prompt: "hidden",
          },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);
    const response = await client.getAgentPresentationPreferences();
    expect(response.preferences.toolInvocation).toBe("detailed");
    await client.setAgentPresentationPreferences(3, response.preferences);

    expect(requests).toEqual([
      { operation: "agent_presentation_preferences", payload: {} },
      {
        operation: "set_agent_presentation_preferences",
        payload: {
          expected_revision: 3,
          preferences: {
            reasoning: "shown",
            tool_invocation: "detailed",
            tool_output: "full",
            prompt: "hidden",
          },
        },
      },
    ]);
  });

  it("reads upstream model rows and sends revision-checked selections", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        if (operation === "model_library") {
          return {
            models: [
              {
                id: "rpi1:10:provider-a12:wire-model-a",
                display_name: "Model A",
                provider_id: "provider-a",
                provider_model_id: "wire-model-a",
                thinking_levels: ["extended", "off", "high"],
                metadata_snapshot: {
                  schema_version: "rpi-safe-model-snapshot-v1",
                  api_id: "anthropic-messages",
                  reasoning: true,
                  input: ["text", "image"],
                  cost: {
                    input: 3,
                    output: 15,
                    cache_read: 0.3,
                    cache_write: 3.75,
                    tiers: [
                      {
                        input: 2,
                        output: 10,
                        cache_read: 0.2,
                        cache_write: 2.5,
                        input_tokens_above: 100000,
                      },
                    ],
                  },
                  cost_configured: true,
                  context_window: 200000,
                  max_tokens: 8192,
                },
              },
              {
                id: "rpi1:10:provider-b12:wire-model-b",
                display_name: "Model B",
                provider_id: "provider-b",
                provider_model_id: "wire-model-b",
                thinking_levels: ["high", "off"],
                metadata_snapshot: {
                  schema_version: "rpi-safe-model-snapshot-v1",
                  api_id: "openai-responses",
                  reasoning: true,
                  input: ["text"],
                  cost: {
                    input: 1,
                    output: 5,
                    cache_read: 0,
                    cache_write: 0,
                    tiers: [],
                  },
                  cost_configured: true,
                  context_window: 128000,
                  max_tokens: 4096,
                },
              },
            ],
            provider_readiness: [
              {
                provider_id: "provider-a",
                model_id: "wire-model-a",
                credential: "available",
              },
            ],
          } as T;
        }
        return {
          revision: 5,
          provider_id: "provider-a",
          model_id: "model-a",
          thinking_level: "extended",
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    const library = await client.getModelLibrary();
    expect(library.models[0]?.id).toBe("rpi1:10:provider-a12:wire-model-a");
    expect(library.models[0]?.providerId).toBe("provider-a");
    expect(library.models[0]?.metadataSnapshot?.reasoning).toBe(true);
    expect(library.models[0]?.metadataSnapshot?.input).toEqual(["text", "image"]);
    expect(library.models[0]?.modelId).toBe("wire-model-a");
    expect(library.providerReadiness).toEqual([
      {
        providerId: "provider-a",
        modelId: "wire-model-a",
        credential: "available",
      },
    ]);
    expect(library.models[0]?.thinkingLevels).toEqual(["extended", "off", "high"]);
    expect(
      library.models.every((model) =>
        model.thinkingLevels.every((level) => typeof level === "string"),
      ),
    ).toBe(true);
    expect(library.models[1]?.thinkingLevels).toEqual(["high", "off"]);
    expect(library.models.map((model) => model.thinkingLevels)).toEqual([
      ["extended", "off", "high"],
      ["high", "off"],
    ]);
    expect(library.models[0]?.metadataSnapshot?.cost.tiers).toEqual([
      {
        input: 2,
        output: 10,
        cacheRead: 0.2,
        cacheWrite: 2.5,
        inputTokensAbove: 100000,
      },
    ]);
    const selection = await client.setSessionModel(
      "session-a" as never,
      4,
      "provider-a",
      "model-a",
    );
    await client.setSessionThinkingLevel("session-a" as never, 5, "extended");

    expect(library.models[0]?.thinkingLevels[0]).toBe("extended");
    expect(library.models).toHaveLength(2);
    expect(selection).toMatchObject({
      revision: 5,
      providerId: "provider-a",
      modelId: "model-a",
    });
    expect(requests).toEqual([
      { operation: "model_library", payload: {} },
      {
        operation: "session_model_selection",
        payload: {
          session: { id: "session-a" },
          expected_revision: 4,
          provider_id: "provider-a",
          model_id: "model-a",
        },
      },
      {
        operation: "session_thinking_selection",
        payload: {
          session: { id: "session-a" },
          expected_revision: 5,
          thinking_level: "extended",
        },
      },
    ]);
  });
});
