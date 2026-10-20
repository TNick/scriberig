/** @file Defines safe model library and execution selection protocol values. */
/** One supported input modality in the upstream safe metadata snapshot. */
export type ModelInputModality = "text" | "image";
/** Typed pricing metadata copied from the upstream safe model snapshot. */
export interface ModelCostSnapshot {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  tiers?: readonly ModelCostTierSnapshot[];
}
/** One upstream pricing tier. */
export interface ModelCostTierSnapshot {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  inputTokensAbove: number;
}
/** Display-safe fields from the versioned upstream model snapshot. */
export interface ModelMetadataSnapshot {
  schemaVersion: string;
  apiId: string;
  reasoning: boolean;
  input: readonly ModelInputModality[];
  cost: ModelCostSnapshot;
  costConfigured: boolean;
  contextWindow: number;
  maxTokens: number;
}
/** Provider model descriptor without resolved credential values. */
export interface ModelDescriptor {
  id: string;
  displayName: string;
  providerId: string;
  modelId: string;
  thinkingLevels: readonly string[];
  metadataSnapshot?: ModelMetadataSnapshot;
}
/** Credential-safe upstream-derived model display rows. */
export interface ModelLibraryResponse {
  models: readonly ModelDescriptor[];
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
