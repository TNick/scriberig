/** @file Defines safe model library and execution selection protocol values. */
/** One thinking capability advertised by a model. */
export interface ThinkingLevelDescriptor {
  id: string;
  label: string;
  description?: string;
  order: number;
}
/** Safe generation capabilities advertised by a model. */
export interface ModelCapabilities {
  streaming: boolean;
  toolCalls: boolean;
  structuredOutput: boolean;
}
/** Provider model descriptor without resolved credential values. */
export interface ModelDescriptor {
  id: string;
  displayName: string;
  providerId: string;
  modelId: string;
  capabilities: ModelCapabilities;
  thinkingLevels: readonly ThinkingLevelDescriptor[];
  defaultThinkingLevel?: string;
}
/** Safe reference to a configured credential. */
export interface CredentialRef {
  id: string;
}
/** Curated model entry clients may select. */
export interface ModelProfile {
  id: string;
  displayName: string;
  model: string;
  connection: string;
  credential?: CredentialRef;
  defaultThinkingLevel?: string;
  enabled: boolean;
}
/** Credential-safe descriptors and curated model profiles. */
export interface ModelLibraryResponse {
  models: readonly ModelDescriptor[];
  profiles: readonly ModelProfile[];
}
/** Session model mutation with its independent optimistic revision. */
export interface SessionModelSelectionRequest {
  session: { id: string };
  expectedRevision: number;
  providerId: string;
  modelId: string;
}
/** Session thinking mutation with its independent optimistic revision. */
export interface SessionThinkingSelectionRequest {
  session: { id: string };
  expectedRevision: number;
  thinkingLevel: string;
}
