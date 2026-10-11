/** @file Defines runtime capability discovery values. */
import type { TypeId } from "./ids";
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
}
