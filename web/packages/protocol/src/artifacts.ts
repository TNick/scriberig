/** @file Defines bounded rich-artifact inspection wire contracts. */

/** Exact artifact capability labels shared by inspection and editor surfaces. */
export type ArtifactEditCapability =
  | "unsupported"
  | "inspect_only"
  | "read_only_derived"
  | "lossless_editable"
  | "loss_aware_editable"
  | "editable_with_opaque_preservation"
  | "restricted_external_provider_edit";

/** Requests a bounded artifact summary page for one project. */
export interface ArtifactInspectRequest {
  project: string;
  offset: number;
  limit: number;
}
/** Requests one bounded page of exact members for an artifact. */
export interface ArtifactMembersRequest {
  project: string;
  artifact: string;
  offset: number;
  limit: number;
}
/** Describes an exact filesystem or container-internal artifact member. */
export interface ArtifactMemberSummary {
  key: string;
  projectPath?: string;
  containerPart?: string;
  digest: string;
}
/** Represents an explicit external artifact reference. */
export interface ArtifactReference {
  kind: string;
  target: string;
}
/** Classifies whether live artifact members match their persisted baseline. */
export type ArtifactSynchronizationStatus = "in-sync" | "modified" | "missing" | "unavailable";
/** Summarizes artifact identity, source evidence, and fidelity without source bytes. */
export interface ArtifactSummary {
  id: string;
  logicalKey: string;
  kind: string;
  adapter: string;
  adapterVersion?: string;
  scanId: string;
  baselineDigest: string;
  observedDigest?: string;
  synchronizationStatus: ArtifactSynchronizationStatus;
  fidelity: string;
  opaquePreservation: string;
  diagnostic?: string;
  references: ArtifactReference[];
  members: ArtifactMemberSummary[];
  memberCount: number;
}
/** Returns bounded physical and package-internal member evidence. */
export interface ArtifactMembersResponse {
  project: string;
  artifact: string;
  members: ArtifactMemberSummary[];
  total: number;
  nextOffset?: number;
}
/** Returns a bounded page of artifact summaries. */
export interface ArtifactInspectResponse {
  project: string;
  artifacts: ArtifactSummary[];
  total: number;
  nextOffset?: number;
}
