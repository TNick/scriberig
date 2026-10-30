/** @file Renders snapshot-bound CAD layers, geometry, and semantic candidates. */
import { useEffect, useMemo, useState, type ReactElement } from "react";
import type { CadDrawingInspectResponse, CadEntitySummary } from "@scriberig/protocol";
import type { ProjectCandidateApplyResult } from "@scriberig/protocol";
import type { CadPageController, ProjectCandidateClient } from "@scriberig/model";
import { ProjectCandidateController } from "@scriberig/model";
import { CadGeometryCanvas, type CadProjectedEntitySelection } from "./CadGeometryCanvas";
import { CadEntityView } from "./cad-drawing-candidates";
import { ProjectCandidateReview } from "./ProjectCandidateReview";

const PAGE_SIZE = 100;

/** Reads and renders semantic CAD drawing data through the shared model.
 * @param props Exact-snapshot CAD controller.
 * @returns Drawing metadata, layers, bounded entities, and safe read errors.
 */
export function CadDrawingView({
  controller,
  client,
  onSelectEntity,
  onCandidateApplied,
}: {
  /** Exact-snapshot CAD controller. */ controller: CadPageController;
  /** Public client for the reusable durable candidate review workflow. */
  client: ProjectCandidateClient;
  /** Publishes semantic entity selection and its exact projection bounds. */
  onSelectEntity?: (selection: CadProjectedEntitySelection) => void;
  /** Publishes the exact successor snapshot after materialization. */
  onCandidateApplied?: (result: ProjectCandidateApplyResult) => void;
}): ReactElement {
  const [drawing, setDrawing] = useState<CadDrawingInspectResponse | undefined>(
    controller.currentDrawing(),
  );
  const [entities, setEntities] = useState<CadEntitySummary[]>(
    controller.currentEntities()?.entities ?? [],
  );
  const [offset, setOffset] = useState(0);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(drawing === undefined);
  const [candidateId, setCandidateId] = useState<string>();
  const [candidateSummary, setCandidateSummary] = useState<string>();
  const candidateController = useMemo(
    () =>
      candidateId === undefined ? undefined : new ProjectCandidateController(client, candidateId),
    [candidateId, client],
  );

  useEffect(() => {
    const abort = new AbortController();
    setLoading(true);
    setError(undefined);
    void Promise.all([
      controller.readDrawing(abort.signal),
      controller.readEntities(0, PAGE_SIZE, abort.signal),
    ])
      .then(([summary, page]) => {
        if (abort.signal.aborted) return;
        setDrawing(summary);
        setEntities(page.entities);
        setOffset(0);
        setTotal(page.total);
        setHasMore(page.hasMore);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      })
      .finally(() => {
        if (!abort.signal.aborted) setLoading(false);
      });
    return () => abort.abort();
  }, [controller]);

  /** Loads one bounded exact-snapshot CAD entity page.
   * @param nextOffset Zero-based start position for the requested page.
   * @returns Promise that settles after page state and loading status update.
   */
  async function loadPage(nextOffset: number): Promise<void> {
    setLoading(true);
    setError(undefined);
    try {
      const page = await controller.readEntities(nextOffset, PAGE_SIZE);
      setEntities(page.entities);
      setOffset(nextOffset);
      setTotal(page.total);
      setHasMore(page.hasMore);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section aria-label="Semantic CAD drawing">
      {loading ? <p role="status">Loading drawing…</p> : null}
      {error !== undefined ? <p role="alert">{error}</p> : null}
      {drawing ? <DrawingSummary drawing={drawing} /> : null}
      {drawing?.layers.map((layer) => (
        <p key={layer.id} data-layer-id={layer.id}>
          Layer {layer.name} {layer.visible ? "(visible)" : "(hidden)"}
          {layer.locked ? " (locked)" : ""}
        </p>
      ))}
      <CadGeometryCanvas
        entities={entities}
        visibleLayerIds={
          new Set(drawing?.layers.filter((layer) => layer.visible).map((layer) => layer.id) ?? [])
        }
        onSelectEntity={onSelectEntity}
      />
      <section aria-label="CAD entities">
        {entities.map((entity) => (
          <CadEntityView
            key={entity.id}
            controller={controller}
            entity={entity}
            layers={drawing?.layers ?? []}
            editable={drawing?.complete === true}
            onCandidateCreated={(candidate, summary) => {
              setCandidateId(candidate.candidateId);
              setCandidateSummary(summary);
            }}
          />
        ))}
      </section>
      {candidateController ? (
        <ProjectCandidateReview
          key={candidateId}
          controller={candidateController}
          {...(candidateSummary === undefined ? {} : { summary: candidateSummary })}
          onApplied={(result) => onCandidateApplied?.(result)}
        />
      ) : null}
      <nav aria-label="CAD entity pages">
        <button
          type="button"
          disabled={offset === 0 || loading}
          onClick={() => void loadPage(Math.max(0, offset - PAGE_SIZE))}
        >
          Previous
        </button>
        <span>
          {total === 0
            ? "0 entities"
            : `${offset + 1}–${Math.min(offset + entities.length, total)} of ${total}`}
        </span>
        <button
          type="button"
          disabled={!hasMore || loading}
          onClick={() => void loadPage(offset + PAGE_SIZE)}
        >
          Next
        </button>
      </nav>
    </section>
  );
}

/** Renders exact drawing metadata and import completeness.
 * @param props Typed drawing summary.
 * @returns Accessible metadata and diagnostics.
 */
function DrawingSummary({
  drawing,
}: {
  /** Typed drawing metadata. */ drawing: CadDrawingInspectResponse;
}): ReactElement {
  return (
    <header>
      <h2>Drawing</h2>
      <p>Units: {drawing.units}</p>
      {drawing.coordinateReference ? (
        <p>Coordinate reference: {drawing.coordinateReference}</p>
      ) : null}
      {!drawing.complete ? <p role="status">This drawing has unsupported source content.</p> : null}
      {drawing.diagnostics.map((message, index) => (
        <p key={`${index}-${message}`} role="note">
          {message}
        </p>
      ))}
    </header>
  );
}
