/** @file Normalizes shared presentation preferences from the Rust wire format. */

import type { AgentPresentationPreferencesResponse } from "@scriberig/protocol";
import type { AgentPresentationPreferences } from "@scriberig/protocol";
import type { SemanticTransport } from "./transport";

/** Normalizes one preference response while preserving its mutation revision.
 * @param value Unknown server response.
 * @returns Client-facing shared preference snapshot.
 * @throws When the response contains invalid values.
 */
export function normalizePresentationPreferencesResponse(
  value: unknown,
): AgentPresentationPreferencesResponse {
  if (typeof value !== "object" || value === null) {
    throw new Error("server returned invalid presentation preferences");
  }
  const root = value as Record<string, unknown>;
  const source = root.preferences as Record<string, unknown> | undefined;
  if (typeof root.revision !== "number" || source === undefined) {
    throw new Error("server returned incomplete presentation preferences");
  }
  const preferences = {
    reasoning: source.reasoning,
    toolInvocation: source.tool_invocation,
    toolOutput: source.tool_output,
    prompt: source.prompt,
  };
  if (
    !["hidden", "shown"].includes(String(preferences.reasoning)) ||
    !["simple", "detailed"].includes(String(preferences.toolInvocation)) ||
    !["hidden", "summary", "full"].includes(String(preferences.toolOutput)) ||
    !["hidden", "shown"].includes(String(preferences.prompt))
  ) {
    throw new Error("server returned invalid presentation preference values");
  }
  return {
    revision: root.revision,
    preferences: preferences as AgentPresentationPreferencesResponse["preferences"],
  };
}

/** Reads normalized shared presentation defaults through one semantic transport.
 * @param transport Injected protocol transport.
 * @param signal Optional cancellation signal.
 * @returns Revisioned shared defaults.
 */
export function readPresentationPreferences(
  transport: SemanticTransport,
  signal?: AbortSignal,
): Promise<AgentPresentationPreferencesResponse> {
  return transport
    .request("agent_presentation_preferences", {}, signal)
    .then(normalizePresentationPreferencesResponse);
}

/** Sends a revision-checked replacement using Rust wire field names.
 * @param transport Injected protocol transport.
 * @param expectedRevision Last observed shared preference revision.
 * @param preferences Complete client-facing preference values.
 * @param signal Optional cancellation signal.
 * @returns Updated revisioned defaults.
 */
export function replacePresentationPreferences(
  transport: SemanticTransport,
  expectedRevision: number,
  preferences: AgentPresentationPreferences,
  signal?: AbortSignal,
): Promise<AgentPresentationPreferencesResponse> {
  return transport
    .request(
      "set_agent_presentation_preferences",
      {
        expected_revision: expectedRevision,
        preferences: {
          reasoning: preferences.reasoning,
          tool_invocation: preferences.toolInvocation,
          tool_output: preferences.toolOutput,
          prompt: preferences.prompt,
        },
      },
      signal,
    )
    .then(normalizePresentationPreferencesResponse);
}
