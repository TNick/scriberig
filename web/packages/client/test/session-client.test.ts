/** @file Verifies authoritative session and branch projections. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";

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

  it("reads safe model capabilities and sends revision-checked selections", async () => {
    const requests: { operation: string; payload: unknown }[] = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        requests.push({ operation, payload });
        if (operation === "model_library") {
          return {
            models: [
              {
                id: "model-a",
                display_name: "Model A",
                provider_profile: "provider-a",
                provider_model_id: "wire-model-a",
                capabilities: { streaming: true, tool_calls: true, structured_output: false },
                thinking_levels: [{ id: "extended", label: "Extended", order: 0 }],
                default_thinking_level: "extended",
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
          } as T;
        }
        return {
          revision: 5,
          model_profile: "profile-a",
          model: "model-a",
          thinking_level: "extended",
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    const library = await client.getModelLibrary();
    const selection = await client.setSessionModel("session-a" as never, 4, "profile-a");
    await client.setSessionThinkingLevel("session-a" as never, 5, "extended");

    expect(library.models[0]?.thinkingLevels[0]?.id).toBe("extended");
    expect(library.profiles[0]?.credential).toEqual({ id: "secret-reference-only" });
    expect(selection).toMatchObject({ revision: 5, modelProfile: "profile-a" });
    expect(requests).toEqual([
      { operation: "model_library", payload: {} },
      {
        operation: "session_model_selection",
        payload: {
          session: { id: "session-a" },
          expected_revision: 4,
          model_profile: "profile-a",
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

  it("normalizes Rust session reads and derives branch views", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          session: { id: "session-1" },
          execution: {
            revision: 3,
            model_profile: "profile-a",
            model: "model-a",
            thinking_level: "careful",
          },
          branches: [
            {
              id: "root",
              title: "Root",
              status: "Completed",
              children: ["child"],
              turns: [{ id: "turn-1", ordinal: 1, has_human_input: true, has_agent_output: false }],
              activity: [
                {
                  id: "tool-call-1",
                  session: "session-1",
                  sequence: 2,
                  branch: "root",
                  kind: "ToolInvocation",
                  state: "Completed",
                  summary: "Tool: search",
                  tool_name: "search",
                  tool_arguments: { query: "architecture", token: "[REDACTED]" },
                },
                {
                  id: "tool-result-1",
                  session: "session-1",
                  sequence: 3,
                  branch: "root",
                  parent: "tool-call-1",
                  kind: "ToolResult",
                  state: "Completed",
                  summary: "Tool completed",
                  durable_ref: "tool-result-1",
                },
              ],
            },
            {
              id: "child",
              parent: "root",
              title: "Child",
              status: "Completed",
              children: [],
              turns: [],
              result: { id: "result-1", references: ["migration-1"] },
            },
          ],
        }) as T,
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    const session = await client.getSession("session-1" as never);
    expect(session.execution).toEqual({
      revision: 3,
      modelProfile: "profile-a",
      model: "model-a",
      thinkingLevel: "careful",
    });
    expect(session.branches[0]?.turns[0]?.hasHumanInput).toBe(true);
    expect(await client.getBranch("session-1" as never, "child" as never)).toMatchObject({
      branchId: "child",
      status: "completed",
    });
    expect(await client.listBranchChildren("session-1" as never, "root" as never)).toMatchObject({
      items: [{ branchId: "child" }],
    });
    expect(await client.listBranchActivity("session-1" as never, "root" as never)).toMatchObject({
      items: [
        {
          id: "tool-call-1",
          kind: "tool",
          activityKind: "ToolInvocation",
          toolArguments: { query: "architecture", token: "[REDACTED]" },
        },
        {
          id: "tool-result-1",
          kind: "tool",
          activityKind: "ToolResult",
          parentId: "tool-call-1",
          handles: ["tool-result-1"],
        },
      ],
    });
    expect(await client.getBranchResult("session-1" as never, "result-1")).toEqual({
      branchId: "child",
      resultId: "result-1",
      handles: ["migration-1"],
    });
  });
});
