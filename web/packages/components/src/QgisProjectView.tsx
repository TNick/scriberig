/** @file Renders QGIS references and candidate-only project ordering controls. */
import { useState, type ReactElement } from "react";
import type { QgisLayerReferenceView, QgisProjectResponse } from "@scriberig/protocol";
import type { QgisProjectController } from "@scriberig/model";
import { summarizeQgisLayerOrder, summarizeQgisLayerRename } from "./semantic-change-summary";

/** Displays semantic QGIS references and candidate-based project layer ordering.
 *
 * Public members: the component accepts the exact project, its model controller,
 * and browse/candidate/error callbacks.
 * Private members: local rename target and draft name.
 */
export function QgisProjectView({
  project,
  controller,
  canBrowse,
  onBrowse,
  onCandidate,
  onError,
}: {
  /** Exact-snapshot QGIS project model. */
  project: QgisProjectResponse;
  /** Shared semantic controller already read at this project's snapshot. */
  controller: QgisProjectController;
  /** Determines whether a linked layer can be opened in the current browser. */
  canBrowse: (reference: QgisLayerReferenceView) => boolean;
  /** Opens a selected reference's resolved semantic layer. */
  onBrowse: (reference: QgisLayerReferenceView) => void;
  /** Publishes the shared candidate identity and readable semantic summary. */
  onCandidate: (candidate: string, summary: string) => void;
  /** Publishes a safe user-facing operation error. */
  onError: (message: string) => void;
}): ReactElement {
  const [renameReference, setRenameReference] = useState<string>();
  const [renameName, setRenameName] = useState("");
  const canReorder =
    project.groups.length === 1 && project.groups[0]?.layers.length === project.layers.length;

  return (
    <section aria-label="QGIS layer references">
      <h2>QGIS layer references</h2>
      {project.layers.map((reference, index) => (
        <div key={reference.id}>
          <div className="qgis-layer-actions">
            <button disabled={!canBrowse(reference)} onClick={() => onBrowse(reference)}>
              {reference.name} · Layer reference {reference.id}
            </button>
            <button
              type="button"
              disabled={!canReorder || index === 0}
              aria-label={`Move ${reference.name} earlier`}
              onClick={() => void proposeMove(index, -1)}
            >
              Move up
            </button>
            <button
              type="button"
              disabled={!canReorder || index === project.layers.length - 1}
              aria-label={`Move ${reference.name} later`}
              onClick={() => void proposeMove(index, 1)}
            >
              Move down
            </button>
            <button
              type="button"
              onClick={() => {
                setRenameReference(reference.id);
                setRenameName(reference.name);
              }}
            >
              Rename reference
            </button>
          </div>
          {renameReference === reference.id ? (
            <div className="qgis-layer-rename">
              <label>
                New QGIS layer name
                <input value={renameName} onChange={(event) => setRenameName(event.target.value)} />
              </label>
              <button
                type="button"
                disabled={renameName.trim().length === 0}
                onClick={() => void proposeRename(reference)}
              >
                Propose rename
              </button>
            </div>
          ) : null}
          <p>
            {reference.layer === undefined
              ? `Unresolved reference: ${reference.resolutionDiagnostic ?? reference.dataSource}`
              : canBrowse(reference)
                ? `Resolved GeoLayer ${reference.layer} from ${reference.resolvedPath}`
                : `Resolved GeoLayer ${reference.layer} is read only: ${reference.resolutionDiagnostic ?? reference.resolvedPath ?? reference.dataSource}`}
          </p>
          <p aria-label={`Project presentation for ${reference.name}`}>
            Project presentation: {reference.visible ? "visible" : "hidden"}
            {reference.renderer === undefined ? "" : ` · renderer kind ${reference.renderer}`}
            {reference.scaleRange === undefined ? "" : ` · scale range ${reference.scaleRange}`}
            {reference.renderer === undefined
              ? ""
              : " · renderer metadata is retained as project configuration; the feature map uses neutral styling"}
          </p>
        </div>
      ))}
      {!canReorder ? (
        <p role="note">Layer ordering is available for flat QGIS project trees.</p>
      ) : null}
    </section>
  );

  /** Proposes moving one reference by one position in project order.
   * @param index Current zero-based layer index.
   * @param direction Signed one-position movement.
   * @returns Completion after the candidate request settles.
   */
  async function proposeMove(index: number, direction: -1 | 1): Promise<void> {
    const target = index + direction;
    if (target < 0 || target >= project.layers.length || !canReorder) return;
    const next = [...project.layers];
    [next[index], next[target]] = [next[target]!, next[index]!];
    try {
      const candidate = await controller.reorderLayers(next.map((layer) => layer.id));
      onCandidate(candidate.candidate, summarizeQgisLayerOrder(project.layers, next));
    } catch (reason) {
      onError(String(reason));
    }
  }

  /** Proposes the selected QGIS layer-reference rename.
   * @param reference Exact semantic reference to rename.
   * @returns Completion after the candidate request settles.
   */
  async function proposeRename(reference: QgisLayerReferenceView): Promise<void> {
    try {
      const candidate = await controller.renameReference(reference.id, renameName);
      onCandidate(
        candidate.candidate,
        summarizeQgisLayerRename(reference.id, reference.name, renameName),
      );
      setRenameReference(undefined);
    } catch (reason) {
      onError(String(reason));
    }
  }
}
