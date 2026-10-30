/** @file Verifies safe upstream model library normalization. */
import { describe, expect, it } from "vitest";
import { normalizeModelLibrary } from "../src/model-views.js";

describe("session client model projections", () => {
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
  });
});
