/** @file Verifies authoritative session and branch projections. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
import { normalizeModelLibrary } from "../src/model-views.js";
import { normalizeSession } from "../src/session-views.js";

describe("session client projections", () => {
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
                authentication: "accepted",
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
        authentication: "accepted",
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

  it("normalizes scalar upstream thinking ids without restoring retired catalog fields", () => {
    const library = normalizeModelLibrary({
      models: [
        {
          id: "rpi1:10:provider-a12:wire-model-a",
          display_name: "Model A",
          provider_id: "provider-a",
          provider_model_id: "wire-model-a",
          capabilities: {
            streaming: true,
            tool_calls: true,
            structured_output: false,
          },
          thinking_levels: ["extended"],
          default_thinking_level: "extended",
          metadata_snapshot: {
            schema_version: "rpi-safe-model-snapshot-v1",
            api_id: "openai-completions",
            reasoning: true,
            input: ["text"],
            cost: {
              input: 1,
              output: 2,
              cache_read: 0,
              cache_write: 0,
            },
            cost_configured: true,
            context_window: 8192,
            max_tokens: 2048,
          },
        },
      ],
      profiles: [
        {
          id: "profile-a",
          display_name: "Primary",
          model: "model-a",
          connection: "connection-a",
          credential: { id: "secret-reference-only" },
          enabled: true,
        },
      ],
    });

    expect(library.models).toEqual([
      {
        id: "rpi1:10:provider-a12:wire-model-a",
        displayName: "Model A",
        providerId: "provider-a",
        modelId: "wire-model-a",
        thinkingLevels: ["extended"],
        metadataSnapshot: {
          schemaVersion: "rpi-safe-model-snapshot-v1",
          apiId: "openai-completions",
          reasoning: true,
          input: ["text"],
          cost: {
            input: 1,
            output: 2,
            cacheRead: 0,
            cacheWrite: 0,
            tiers: [],
          },
          costConfigured: true,
          contextWindow: 8192,
          maxTokens: 2048,
        },
      },
    ]);
    expect(library).not.toHaveProperty("profiles");
    expect(() =>
      normalizeModelLibrary({
        models: [
          {
            id: "wrong-row-id",
            display_name: "Model A",
            provider_id: "provider-a",
            provider_model_id: "wire-model-a",
            thinking_levels: [],
            metadata_snapshot: {
              schema_version: "rpi-safe-model-snapshot-v1",
              api_id: "openai-completions",
              reasoning: true,
              input: ["text"],
              cost: { input: 0, output: 0, cache_read: 0, cache_write: 0 },
              cost_configured: true,
              context_window: 8192,
              max_tokens: 2048,
            },
          },
        ],
      }),
    ).toThrow("model row id does not match provider/model identity");
    expect(() =>
      normalizeModelLibrary({
        models: [
          {
            id: "rpi1:5:p😀1:m",
            display_name: "Unicode model",
            provider_id: "p😀",
            provider_model_id: "m",
            thinking_levels: [],
            metadata_snapshot: {
              schema_version: "rpi-safe-model-snapshot-v1",
              api_id: "openai-completions",
              reasoning: true,
              input: ["text"],
              cost: { input: 0, output: 0, cache_read: 0, cache_write: 0 },
              cost_configured: true,
              context_window: 8192,
              max_tokens: 2048,
            },
          },
        ],
      }),
    ).not.toThrow();
    expect(() =>
      normalizeModelLibrary({
        models: [
          {
            id: "rpi1:10:provider-a12:wire-model-a",
            display_name: "Model A",
            provider_id: "provider-a",
            provider_model_id: "wire-model-a",
            thinking_levels: [],
            metadata_snapshot: {
              schema_version: "rpi-safe-model-snapshot-v2",
              cost: {},
            },
          },
        ],
      }),
    ).toThrow("unsupported model metadata schema version");
    expect(() =>
      normalizeModelLibrary({
        models: [
          {
            id: "model-a",
            display_name: "Model A",
            provider_profile: "retired-profile",
            provider_model_id: "wire-model-a",
            thinking_levels: [],
            metadata_snapshot: {},
          },
        ],
      }),
    ).toThrow("missing provider id");
    expect(() =>
      normalizeModelLibrary({
        models: [
          {
            id: "rpi1:10:provider-a12:wire-model-a",
            display_name: "Model A",
            provider_id: "provider-a",
            provider_model_id: "wire-model-a",
            thinking_levels: [],
            metadata_snapshot: {
              schema_version: "rpi-safe-model-snapshot-v1",
              api_id: "openai-completions",
              reasoning: false,
              input: ["text"],
              cost: { input: 0, output: 0, cache_read: 0, cache_write: 0 },
              cost_configured: false,
              context_window: 8192,
              max_tokens: 2048,
            },
          },
        ],
        provider_readiness: [
          {
            provider_id: "provider-a",
            model_id: "wire-model-a",
            credential: "available",
            authentication: "maybe",
          },
        ],
      }),
    ).toThrow("invalid provider authentication status");
  });
});
