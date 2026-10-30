/** @file Provides paged artifact catalog and member reads over the shared transport. */
import type {
  ArtifactInspectRequest,
  ArtifactInspectResponse,
  ArtifactMembersRequest,
  ArtifactMembersResponse,
} from "@scriberig/protocol";
import {
  normalizeArtifactInspectResponse,
  normalizeArtifactMembersResponse,
} from "./artifact-views";
import type { SemanticTransport } from "./transport";

/** Owns bounded reads for persisted project artifact evidence.
 *
 * Public members: constructor and artifact inspection methods.
 * Private members: none.
 */
export class ArtifactClientOperations {
  /** Creates artifact operations over an injected transport.
   * @param transport Protocol transport with artifact inspection operations.
   */
  constructor(protected readonly transport: SemanticTransport) {}

  /** Reads one bounded logical artifact summary page.
   * @param request Project identity and page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Bounded artifact metadata page.
   */
  inspectArtifacts(
    request: ArtifactInspectRequest,
    signal?: AbortSignal,
  ): Promise<ArtifactInspectResponse> {
    return this.transport
      .request("artifact_inspect", request, signal)
      .then(normalizeArtifactInspectResponse);
  }

  /** Reads one bounded page of exact physical and package-internal members.
   * @param request Exact project, artifact, and page bounds.
   * @param signal Optional request cancellation signal.
   * @returns Exact member evidence with continuation metadata.
   */
  inspectArtifactMembers(
    request: ArtifactMembersRequest,
    signal?: AbortSignal,
  ): Promise<ArtifactMembersResponse> {
    return this.transport
      .request("artifact_members", request, signal)
      .then(normalizeArtifactMembersResponse);
  }
}
