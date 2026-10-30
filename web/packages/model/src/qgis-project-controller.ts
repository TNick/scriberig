/** @file Coordinates exact-snapshot QGIS project inspection and rename candidates. */
import type {
  QgisLayerOrderResponse,
  QgisLayerRenameResponse,
  QgisProjectResponse,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Controls semantic QGIS project references and candidate-only display-name edits.
 *
 * Public members: `constructor`, `read`, `renameReference`, and `currentProject`.
 * Private members: client, project identity, source path, exact snapshot, and loaded project.
 */
export class QgisProjectController {
  private projectView?: QgisProjectResponse;

  /** Creates a controller bound to one exact project source and snapshot.
   * @param client Shared QGIS project and candidate operations.
   * @param project Stable owning project identity.
   * @param path Exact project-relative QGS or QGZ source path.
   * @param snapshot Exact immutable semantic snapshot identity.
   * @throws When the source path is empty or escapes the project root.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      "inspectQgisProject" | "proposeQgisLayerRename" | "proposeQgisLayerOrder"
    >,
    private readonly project: string,
    private readonly path: string,
    private readonly snapshot: string,
  ) {
    if (path.length === 0 || path.startsWith("/") || path.split("/").includes("..")) {
      throw new TypeError("QGIS project path must be project-relative");
    }
  }

  /** Reads and stores the exact semantic project reference tree.
   * @param signal Optional request cancellation signal.
   * @returns Exact-snapshot project semantics.
   * @throws When the response identity does not match this controller.
   */
  async read(signal?: AbortSignal): Promise<QgisProjectResponse> {
    const result = await this.client.inspectQgisProject(
      { project: this.project, path: this.path, snapshot: this.snapshot },
      signal,
    );
    if (
      result.project !== this.project ||
      result.path !== this.path ||
      result.snapshot !== this.snapshot
    ) {
      throw new TypeError("QGIS project identity does not match the requested snapshot");
    }
    this.projectView = result;
    return result;
  }

  /** Proposes a rename for a layer reference present in the loaded exact tree.
   * @param reference Stable reference identity from the loaded tree.
   * @param newName Requested authored display name.
   * @param signal Optional request cancellation signal.
   * @returns Shared project candidate handle.
   * @throws When no exact tree is loaded, the target is absent, or response scope mismatches.
   */
  async renameReference(
    reference: string,
    newName: string,
    signal?: AbortSignal,
  ): Promise<QgisLayerRenameResponse> {
    if (this.projectView === undefined) {
      throw new Error("Read the exact QGIS project before proposing a rename");
    }
    if (!this.projectView.layers.some((layer) => layer.id === reference)) {
      throw new RangeError("QGIS layer reference is absent from the loaded project tree");
    }
    if (newName.trim().length === 0) {
      throw new RangeError("QGIS layer name must contain non-whitespace text");
    }
    const candidate = await this.client.proposeQgisLayerRename(
      { project: this.project, snapshot: this.snapshot, reference, newName },
      signal,
    );
    if (
      candidate.project !== this.project ||
      candidate.baseSnapshot !== this.snapshot ||
      candidate.path !== this.path ||
      candidate.reference !== reference
    ) {
      throw new TypeError("QGIS rename candidate does not match the requested project scope");
    }
    return candidate;
  }

  /** Proposes an exact permutation for a loaded flat project layer tree.
   * @param orderedReferences Complete semantic layer-reference permutation.
   * @param signal Optional request cancellation signal.
   * @returns Shared project candidate handle.
   * @throws When no exact project is loaded, tree shape/order is invalid, or scope mismatches.
   */
  async reorderLayers(
    orderedReferences: readonly string[],
    signal?: AbortSignal,
  ): Promise<QgisLayerOrderResponse> {
    const project = this.projectView;
    if (project === undefined) {
      throw new Error("Read the exact QGIS project before proposing a layer order");
    }
    if (project.groups.length !== 1 || project.groups[0]?.layers.length !== project.layers.length) {
      throw new RangeError("QGIS layer reorder requires one flat root group");
    }
    const known = new Set(project.layers.map((layer) => layer.id));
    const requested = new Set(orderedReferences);
    if (
      known.size !== project.layers.length ||
      requested.size !== orderedReferences.length ||
      known.size !== requested.size ||
      [...known].some((reference) => !requested.has(reference))
    ) {
      throw new RangeError("QGIS layer order must be a complete unique permutation");
    }
    if (project.layers.every((layer, index) => layer.id === orderedReferences[index])) {
      throw new RangeError("QGIS layer order must change the current order");
    }
    const candidate = await this.client.proposeQgisLayerOrder(
      {
        project: this.project,
        snapshot: this.snapshot,
        path: this.path,
        orderedReferences: [...orderedReferences],
      },
      signal,
    );
    if (
      candidate.project !== this.project ||
      candidate.baseSnapshot !== this.snapshot ||
      candidate.path !== this.path
    ) {
      throw new TypeError("QGIS layer-order candidate does not match the requested project scope");
    }
    return candidate;
  }

  /** Returns the currently loaded exact project tree.
   * @returns The loaded project response or `undefined` before a successful read.
   */
  currentProject(): QgisProjectResponse | undefined {
    return this.projectView;
  }
}
