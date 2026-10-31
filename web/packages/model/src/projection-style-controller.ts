/** @file Coordinates projection-style schema, resolution, and settings requests. */
import type {
  ProjectionStyleResolveRequest,
  ProjectionStyleResolveResponse,
  ProjectionStyleSchemaResponse,
  ProjectionStyleSettingRequest,
  ProjectionStyleSettingResponse,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Exposes shared projection-style operations to reusable presentation models.
 *
 * Public members: `schema`, `resolve`, and `set`.
 * Private members: the injected shared semantic client.
 */
export class ProjectionStyleController {
  /** Creates a controller backed by the public semantic client.
   * @param client Client exposing shared projection-style operations.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      "projectionStyleSchema" | "resolveProjectionStyle" | "setProjectionStyle"
    >,
  ) {}

  /** Reads adapter-owned style properties for one projection kind.
   * @param projectionKind Stable adapter projection-kind identity.
   * @param signal Optional request cancellation signal.
   * @returns The adapter schema, or undefined when no schema is declared.
   */
  schema(
    projectionKind: string,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleSchemaResponse | undefined> {
    return this.client.projectionStyleSchema(projectionKind, signal);
  }

  /** Resolves exact style bytes, provenance, and fidelity for one purpose.
   * @param request Project/workspace/view scopes and requested purpose.
   * @param signal Optional request cancellation signal.
   * @returns Shared resolver output without presentation-specific reinterpretation.
   */
  resolve(
    request: ProjectionStyleResolveRequest,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleResolveResponse> {
    return this.client.resolveProjectionStyle(request, signal);
  }

  /** Writes or clears a setting through the shared application operation.
   * @param request Explicit setting scope and optional compare-and-swap revision.
   * @param signal Optional request cancellation signal.
   * @returns The authoritative stored style bytes and revision.
   */
  set(
    request: ProjectionStyleSettingRequest,
    signal?: AbortSignal,
  ): Promise<ProjectionStyleSettingResponse> {
    return this.client.setProjectionStyle(request, signal);
  }
}
