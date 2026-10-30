/** @file Defines runtime capability discovery values. */
import type { TypeId } from "./ids";
import type { EntityId, SnapshotId } from "./ids";
/** Runtime capability descriptor. */
export interface CapabilityDescriptor {
  id: string;
  description: string;
  available: boolean;
  entityType?: TypeId;
}
/** Capability discovery response. */
export interface CapabilitiesResponse {
  protocolMajor: 1;
  capabilities: CapabilityDescriptor[];
  features: string[];
  root?: { entityId: EntityId; snapshotId: SnapshotId };
  cadProviders?: CadProviderCapabilityInfo[];
}

/** Provider availability state for one optional CAD format. */
export type CadProviderAvailability = "available" | "unavailable" | "initializationFailed";

/** Optional CAD package format. */
export type CadProviderFormat = "dwg" | "dxf";

/** Provider guarantee for CAD content outside shared semantics. */
export type CadOpaquePreservation = "preserved" | "unproven" | "notPreserved";

/** Provider-neutral native CAD capability report from the server runtime. */
export interface CadProviderCapabilityInfo {
  format: CadProviderFormat;
  availability: CadProviderAvailability;
  providerId?: string;
  providerVersion?: string;
  canRead: boolean;
  canWrite: boolean;
  canValidate: boolean;
  opaquePreservation: CadOpaquePreservation;
}
