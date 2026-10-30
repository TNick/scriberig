/** @file Coordinates exact-snapshot CAD reads and candidate-only edits. */
import type {
  CadDrawingInspectResponse,
  CadEntitiesResponse,
  CadLayerChangeResponse,
  CadLineEditResponse,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Controls a bounded drawing page and its semantic line edit operation.
 *
 * Public members: snapshot-bound reads, line and layer candidates, and current read models.
 * Private members: client, project, exact snapshot, drawing ID, and latest read models.
 */
export class CadPageController {
  private drawingView?: CadDrawingInspectResponse;
  private entitiesView?: CadEntitiesResponse;

  /** Creates a CAD controller bound to one exact project snapshot.
   * @param client Shared CAD semantic client operations.
   * @param project Stable owning project identity.
   * @param snapshot Exact immutable snapshot identity.
   * @param drawing Stable semantic drawing identity.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      "inspectCadDrawing" | "cadEntities" | "proposeCadLineEdit" | "proposeCadLayerChange"
    >,
    private readonly project: string,
    private readonly snapshot: string,
    private readonly drawing: string,
  ) {}

  /** Reads drawing metadata and semantic layer summaries.
   * @param signal Optional request cancellation signal.
   * @returns Snapshot-bound drawing view.
   */
  async readDrawing(signal?: AbortSignal): Promise<CadDrawingInspectResponse> {
    const view = await this.client.inspectCadDrawing(
      { project: this.project, snapshot: this.snapshot, drawing: this.drawing },
      signal,
    );
    assertIdentity(
      view.project === this.project &&
        view.snapshot === this.snapshot &&
        view.drawing === this.drawing,
      "CAD drawing response identity does not match the requested snapshot",
    );
    this.drawingView = view;
    return view;
  }

  /** Reads one bounded page of semantic drawing entities.
   * @param offset Zero-based entity offset.
   * @param limit Requested entity count from one through five hundred.
   * @param signal Optional request cancellation signal.
   * @returns Snapshot-bound entity page.
   */
  async readEntities(
    offset: number,
    limit: number,
    signal?: AbortSignal,
  ): Promise<CadEntitiesResponse> {
    const page = await this.client.cadEntities(
      { project: this.project, snapshot: this.snapshot, drawing: this.drawing, offset, limit },
      signal,
    );
    assertIdentity(
      page.project === this.project &&
        page.snapshot === this.snapshot &&
        page.drawing === this.drawing,
      "CAD entity page identity does not match the requested snapshot",
    );
    this.entitiesView = page;
    return page;
  }

  /** Proposes new endpoints for a loaded semantic LINE entity.
   * @param entity Stable semantic entity identity in the loaded page.
   * @param start Exact decimal replacement start coordinates.
   * @param end Exact decimal replacement end coordinates.
   * @param signal Optional request cancellation signal.
   * @returns Shared reviewable candidate handle.
   * @throws When the entity is not in the loaded page or is not a LINE.
   */
  replaceLine(
    entity: string,
    start: { x: string; y: string; z: string },
    end: { x: string; y: string; z: string },
    signal?: AbortSignal,
  ): Promise<CadLineEditResponse> {
    if (this.drawingView?.complete !== true) {
      return Promise.reject(new TypeError("CAD drawing fidelity does not permit semantic edits"));
    }
    const target = this.entitiesView?.entities.find((item) => item.id === entity);
    if (!target)
      return Promise.reject(new RangeError("CAD entity is not present in the loaded page"));
    if (!isLineGeometry(target.geometry)) {
      return Promise.reject(new TypeError("CAD candidate operation supports LINE geometry only"));
    }
    return this.client
      .proposeCadLineEdit(
        { project: this.project, snapshot: this.snapshot, entity, start, end },
        signal,
      )
      .then((candidate) => {
        assertCandidateIdentity(candidate, this.project, this.snapshot);
        return candidate;
      });
  }

  /** Proposes moving a loaded entity to an existing unlocked drawing layer.
   * @param entity Stable semantic entity identity in the loaded page.
   * @param destinationLayer Existing semantic layer identity in the same drawing.
   * @param signal Optional request cancellation signal.
   * @returns Shared reviewable candidate handle.
   * @throws When fidelity is incomplete or either semantic identity is unavailable.
   */
  changeLayer(
    entity: string,
    destinationLayer: string,
    signal?: AbortSignal,
  ): Promise<CadLayerChangeResponse> {
    if (this.drawingView?.complete !== true) {
      return Promise.reject(new TypeError("CAD drawing fidelity does not permit semantic edits"));
    }
    const target = this.entitiesView?.entities.find((item) => item.id === entity);
    if (!target) {
      return Promise.reject(new RangeError("CAD entity is not present in the loaded page"));
    }
    const layer = this.drawingView.layers.find((item) => item.id === destinationLayer);
    if (!layer) {
      return Promise.reject(new RangeError("destination CAD layer is not present in this drawing"));
    }
    if (layer.locked) {
      return Promise.reject(new TypeError("destination CAD layer is locked"));
    }
    return this.client
      .proposeCadLayerChange(
        { project: this.project, snapshot: this.snapshot, entity, destinationLayer },
        signal,
      )
      .then((candidate) => {
        assertCandidateIdentity(candidate, this.project, this.snapshot);
        return candidate;
      });
  }

  /** Returns the latest successfully loaded drawing summary.
   * @returns Drawing view or undefined before the first successful read.
   */
  currentDrawing(): CadDrawingInspectResponse | undefined {
    return this.drawingView;
  }

  /** Returns the latest successfully loaded entity page.
   * @returns Entity page or undefined before the first successful read.
   */
  currentEntities(): CadEntitiesResponse | undefined {
    return this.entitiesView;
  }
}

/** Rejects response identity mismatches without replacing the last valid view.
 * @param matches Whether the requested and returned semantic identities match.
 * @param message Error detail for an identity mismatch.
 * @returns Nothing when identities match.
 * @throws When the response belongs to a different project, snapshot, or drawing.
 */
function assertIdentity(matches: boolean, message: string): void {
  if (!matches) throw new TypeError(message);
}

/** Rejects candidate handles returned for a different base context.
 * @param candidate Candidate summary returned by the shared client.
 * @param project Expected project identity.
 * @param snapshot Expected exact base snapshot.
 * @returns Nothing when candidate identities match.
 * @throws When the candidate's project or base snapshot differs.
 */
function assertCandidateIdentity(
  candidate: CadLineEditResponse | CadLayerChangeResponse,
  project: string,
  snapshot: string,
): void {
  if (candidate.project !== project || candidate.baseSnapshot !== snapshot) {
    throw new TypeError("CAD candidate identity does not match the requested base snapshot");
  }
}

/** Checks the serialized semantic geometry discriminator for a LINE.
 * @param geometry Wire-safe typed geometry value.
 * @returns Whether the value contains exactly one LINE geometry variant.
 */
function isLineGeometry(geometry: unknown): boolean {
  if (geometry === null || typeof geometry !== "object" || Array.isArray(geometry)) return false;
  const keys = Object.keys(geometry);
  return keys.length === 1 && keys[0] === "Line";
}
