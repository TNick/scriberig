/** @file Defines safe model library and execution selection protocol values. */
/** One supported input modality in the upstream safe metadata snapshot. */
export type ModelInputModality = "text" | "image";
/** Typed pricing metadata copied from the upstream safe model snapshot. */
export interface ModelCostSnapshot {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  tiers: readonly ModelCostTierSnapshot[];
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
  schemaVersion: "rpi-safe-model-snapshot-v1";
  apiId: string;
  reasoning: boolean;
  input: readonly ModelInputModality[];
  cost: ModelCostSnapshot;
  costConfigured: boolean;
  contextWindow: number;
  maxTokens: number;
}
/** Provider model descriptor without resolved credential values.
 * `id` is the Rust `rpi1` UTF-8 length-prefixed provider/model row key.
 */
export interface ModelDescriptor {
  id: string;
  displayName: string;
  providerId: string;
  modelId: string;
  thinkingLevels: readonly string[];
  metadataSnapshot: ModelMetadataSnapshot;
}
/** Safe provider credential readiness reported by the host. */
export type ProviderCredentialReadiness =
  "not_required" | "available" | "missing" | "denied" | "unsupported" | "catalog_header";
/** Provider-authored result of the latest request for a model. */
export type ProviderAuthenticationStatus = "accepted" | "rejected";
/** Readiness for one provider/model endpoint without credential material.
 *
 * Public members:
 * - `providerId`: Upstream provider identity.
 * - `modelId`: Provider-scoped model identity.
 * - `credential`: Host-projected credential availability.
 * - `authentication`: Optional upstream-authored request outcome.
 *
 * Private members: None.
 */
export interface ProviderReadiness {
  providerId: string;
  modelId: string;
  credential: ProviderCredentialReadiness;
  authentication?: ProviderAuthenticationStatus;
}
/** Credential-safe upstream-derived model display rows. */
export interface ModelLibraryResponse {
  models: readonly ModelDescriptor[];
  providerReadiness?: readonly ProviderReadiness[];
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
/** One sanitized historical execution value retained for explicit recovery. */
export interface LegacyExecutionRecordView {
  path: string;
  value: unknown;
  target?: LegacyExecutionTargetView;
}
/** Stable plan owner for a preserved legacy assignment. */
export interface LegacyExecutionTargetView {
  planId: string;
  nodeId: string;
  revision?: number;
  claim: boolean;
}
/** Reads preserved legacy execution values for one session. */
export interface LegacyExecutionRecordsRequest {
  session: { id: string };
}
/** Maps one exact preserved session execution choice to current upstream IDs. */
export interface LegacyExecutionMappingRequest {
  session: { id: string };
  expectedRevision: number;
  legacyPath: string;
  providerId: string;
  modelId: string;
  thinkingLevel?: string;
}
/** Maps one current legacy plan-node assignment to active upstream IDs. */
export interface LegacyPlanAssignmentMappingRequest {
  session: { id: string };
  legacyPath: string;
  expectedPlanRevision: number;
  providerId: string;
  modelId: string;
  thinkingLevel?: string;
}
