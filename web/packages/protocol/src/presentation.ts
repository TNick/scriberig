/** @file Defines shared agent-console presentation preference values. */

/** Independent defaults for captured agent activity presentation. */
export interface AgentPresentationPreferences {
  /** Safe reasoning summary visibility. */
  reasoning: "hidden" | "shown";
  /** Tool invocation argument detail. */
  toolInvocation: "simple" | "detailed";
  /** Tool result verbosity. */
  toolOutput: "hidden" | "summary" | "full";
  /** Captured model request visibility. */
  prompt: "hidden" | "shown";
}

/** Revisioned shared defaults for an authenticated principal. */
export interface AgentPresentationPreferencesResponse {
  /** Preference mutation revision. */
  revision: number;
  /** Shared defaults. */
  preferences: AgentPresentationPreferences;
}

/** Revision-checked shared preference replacement. */
export interface AgentPresentationPreferencesSetRequest {
  /** Revision observed by the caller. */
  expectedRevision: number;
  /** Complete independent preference values. */
  preferences: AgentPresentationPreferences;
}
