/** @file Defines the public client boundary for trust inspection. */

import type {
  CapabilityDenial,
  TrustDetails,
  TrustEvent,
  TrustSelection,
  TrustSummary,
} from "@scriberig/protocol";

/** Provides trust inspection without exposing transport implementation details. */
export interface TrustClient {
  /** Loads compact trust state for a semantic selection. */
  getTrustSummary(selection: TrustSelection): Promise<TrustSummary>;
  /** Loads provenance, validation, and policy details. */
  getTrustDetails(selection: TrustSelection): Promise<TrustDetails>;
  /** Subscribes to trust changes affecting a selection. */
  subscribeTrust(selection: TrustSelection, listener: (event: TrustEvent) => void): () => void;
}

/** Represents a denied trust request at the client boundary.
 *
 * Public members:
 * - `denial`: Structured denial safe for presentation.
 *
 * Private members: None.
 */
export class TrustAccessDenied extends Error {
  /** Structured denial safe for presentation. */
  readonly denial: CapabilityDenial;

  /** Creates a structured trust access error. */
  constructor(denial: CapabilityDenial) {
    super(denial.reason);
    this.name = "TrustAccessDenied";
    this.denial = denial;
  }
}
