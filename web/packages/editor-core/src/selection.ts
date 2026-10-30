/** @file Defines framework-independent semantic selections and control. */
import type { EntityId, RevisionId, SnapshotId } from "@scriberig/protocol";
import type { ProjectionAnchor } from "@scriberig/protocol";
import { validateProjectionAnchor } from "@scriberig/client";

/** Carries QGIS reference identity while a linked geospatial layer is selected.
 *
 * Public members: `referenceId` and optional `linkedGeoLayerId`.
 * Private members: none.
 */
export interface QgisLayerSelectionContext {
  /** Stable semantic identity of the QGIS project layer reference. */
  referenceId: EntityId;
  /** Resolved shared geospatial layer identity, when the reference resolves. */
  linkedGeoLayerId?: EntityId;
}

/** Exact projection-space bounds for one supported CAD entity.
 *
 * Public members: `minX`, `minY`, `maxX`, and `maxY`.
 * Private members: none.
 */
export interface CadProjectionBounds {
  /** Minimum exact drawing x coordinate. */
  minX: string;
  /** Minimum exact drawing y coordinate. */
  minY: string;
  /** Maximum exact drawing x coordinate. */
  maxX: string;
  /** Maximum exact drawing y coordinate. */
  maxY: string;
}

/** Semantic selection independent of projection coordinates. */
export interface Selection {
  entityId: EntityId;
  snapshotId: SnapshotId;
  revisionId?: RevisionId;
  relationPath?: readonly EntityId[];
  projection?: { key: string; location?: string } | undefined;
  anchor?: ProjectionAnchor | undefined;
  qgisLayer?: QgisLayerSelectionContext | undefined;
}

/** Selects a paragraph or table cell while retaining its owning document context.
 * @param current Current document selection, possibly already focused on a document part.
 * @param partId Stable paragraph or table-cell semantic identity.
 * @param snapshot Exact snapshot containing the selected document part.
 * @returns Semantic part selection with the root document retained in its relation path.
 */
export function selectDocumentPart(
  current: Selection,
  partId: string,
  snapshot: string,
): Selection {
  return {
    entityId: partId as EntityId,
    snapshotId: snapshot as SnapshotId,
    relationPath: [current.relationPath?.[0] ?? current.entityId],
  };
}

/** Selects a QGIS layer reference while keeping its linked GeoLayer separately addressable.
 * @param current Current QGIS project or layer-reference selection.
 * @param referenceId Stable semantic QGIS LayerReference identity.
 * @param snapshot Exact snapshot containing the project and reference.
 * @param linkedGeoLayerId Resolved shared GeoLayer identity, when available.
 * @returns Selection whose primary entity is the QGIS reference and whose relationship context retains the GeoLayer.
 */
export function selectQgisLayerReference(
  current: Selection,
  referenceId: EntityId,
  snapshot: SnapshotId,
  linkedGeoLayerId?: EntityId,
): Selection {
  const projectId = current.relationPath?.[0] ?? current.entityId;
  return {
    entityId: referenceId,
    snapshotId: snapshot,
    relationPath: [projectId],
    qgisLayer: {
      referenceId,
      ...(linkedGeoLayerId === undefined ? {} : { linkedGeoLayerId }),
    },
  };
}

/** Selects the GeoLayer linked by a QGIS reference, preserving project and reference context.
 * @param current Selection of a QGIS LayerReference.
 * @returns Linked GeoLayer selection when the reference resolves; otherwise undefined.
 */
export function selectLinkedQgisGeoLayer(current: Selection): Selection | undefined {
  const context = current.qgisLayer;
  if (context?.linkedGeoLayerId === undefined) return undefined;
  return {
    entityId: context.linkedGeoLayerId,
    snapshotId: current.snapshotId,
    relationPath: [current.relationPath?.[0] ?? current.entityId, context.referenceId],
    qgisLayer: context,
  };
}

/** Selects a CAD entity with exact drawing-space bounds as presentation context.
 * @param entityId Stable semantic CAD entity identity.
 * @param snapshot Exact snapshot containing the drawing and entity.
 * @param drawingId Stable semantic drawing identity.
 * @param space Model, paper, or named drawing-space identity.
 * @param bounds Exact bounds supplied by the semantic geometry projection.
 * @returns Entity selection with CAD bounds kept separate from semantic identity.
 */
export function selectCadEntity(
  entityId: EntityId,
  snapshot: SnapshotId,
  drawingId: EntityId,
  space: string,
  bounds: CadProjectionBounds,
): Selection {
  return {
    entityId,
    snapshotId: snapshot,
    relationPath: [drawingId],
    anchor: { kind: "cadBounds", drawingId, space, ...bounds },
  };
}

/** Selects one semantic feature while retaining its owning layer context.
 * @param featureId Stable semantic geospatial feature identity.
 * @param snapshot Exact snapshot containing the feature and layer.
 * @param layerId Stable semantic layer that owns the selected feature.
 * @returns Feature selection with the layer kept in its relation path.
 */
export function selectGeoFeature(
  featureId: EntityId,
  snapshot: SnapshotId,
  layerId: EntityId,
): Selection {
  return {
    entityId: featureId,
    snapshotId: snapshot,
    relationPath: [layerId],
  };
}

/** Selects a semantic spreadsheet cell with its typed worksheet coordinate anchor.
 * @param cellId Stable semantic cell identity.
 * @param snapshot Exact snapshot containing the cell.
 * @param worksheetId Stable owning worksheet identity.
 * @param row One-based worksheet row.
 * @param column One-based worksheet column.
 * @returns Cell selection whose coordinate remains projection context.
 * @throws When a coordinate or worksheet identity is invalid.
 */
export function selectSpreadsheetCell(
  cellId: EntityId,
  snapshot: SnapshotId,
  worksheetId: EntityId,
  row: number,
  column: number,
): Selection {
  const anchor: ProjectionAnchor = { kind: "spreadsheetCell", worksheetId, row, column };
  validateSpreadsheetAnchor(anchor);
  return {
    entityId: cellId,
    snapshotId: snapshot,
    relationPath: [worksheetId],
    anchor,
  };
}

/** Selects a semantic spreadsheet range on its owning worksheet.
 * @param worksheetId Stable worksheet identity and semantic selection target.
 * @param snapshot Exact snapshot containing the worksheet and cells.
 * @param firstRow First included one-based row.
 * @param firstColumn First included one-based column.
 * @param lastRow Last included one-based row.
 * @param lastColumn Last included one-based column.
 * @returns Worksheet selection with inclusive range anchor context.
 * @throws When range coordinates are invalid or reversed.
 */
export function selectSpreadsheetRange(
  worksheetId: EntityId,
  snapshot: SnapshotId,
  firstRow: number,
  firstColumn: number,
  lastRow: number,
  lastColumn: number,
): Selection {
  const anchor: ProjectionAnchor = {
    kind: "spreadsheetRange",
    worksheetId,
    firstRow,
    firstColumn,
    lastRow,
    lastColumn,
  };
  validateSpreadsheetAnchor(anchor);
  return { entityId: worksheetId, snapshotId: snapshot, anchor };
}

/** Validates a spreadsheet projection anchor before publishing a selection.
 * @param anchor Typed cell or range coordinate anchor.
 * @throws When the wire contract rejects its identity or coordinates.
 */
function validateSpreadsheetAnchor(
  anchor: Extract<ProjectionAnchor, { kind: "spreadsheetCell" | "spreadsheetRange" }>,
): void {
  validateProjectionAnchor(anchor);
}
/** Compares semantic identity while ignoring projection location. @param left First selection. @param right Second selection. @returns Whether semantic references match. */
export function sameSelection(left: Selection | undefined, right: Selection | undefined): boolean {
  return (
    left?.entityId === right?.entityId &&
    left?.snapshotId === right?.snapshotId &&
    left?.revisionId === right?.revisionId
  );
}
/** Controls active selection with subscriptions.
 *
 * Public members: `get`, `set`, `clear`, `subscribe`, `updateProjection`, `updateAnchor`, and `refreshSnapshot`.
 * Private members: current selection and listener set.
 */
export class SelectionController {
  private current: Selection | undefined;
  private readonly listeners = new Set<(selection?: Selection) => void>();
  /** Reads active selection. @returns Active selection. */
  get(): Selection | undefined {
    return this.current;
  }
  /** Sets selection. @param selection New selection. */
  set(selection: Selection): void {
    this.current = selection;
    for (const listener of this.listeners) listener(this.current);
  }
  /** Clears selection. */
  clear(): void {
    this.current = undefined;
    for (const listener of this.listeners) listener(undefined);
  }
  /** Subscribes to changes. @param listener Selection listener. @returns Unsubscribe function. */
  subscribe(listener: (selection?: Selection) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  /** Updates projection coordinates without changing semantic identity. @param projection Projection metadata. */
  updateProjection(projection: Selection["projection"]): void {
    if (this.current) {
      const next = { ...this.current };
      if (projection === undefined) delete next.projection;
      else next.projection = projection;
      this.set(next);
    }
  }
  /** Updates typed domain anchor without changing selected entity identity. @param anchor Domain projection anchor. */
  updateAnchor(anchor: ProjectionAnchor | undefined): void {
    if (this.current) {
      const next = { ...this.current };
      if (anchor === undefined) delete next.anchor;
      else next.anchor = anchor;
      this.set(next);
    }
  }

  /** Rebinds an existing semantic selection to a newer exact snapshot.
   * The caller supplies membership from the refreshed semantic view. A missing
   * entity clears selection; a surviving entity keeps its stable identity while
   * stale revision and projection metadata are discarded.
   * @param snapshotId Exact snapshot represented by the refreshed view.
   * @param entityExists Whether the selected entity still exists in that view.
   * @returns The refreshed selection, or undefined when selection was cleared.
   */
  refreshSnapshot(snapshotId: SnapshotId, entityExists: boolean): Selection | undefined {
    if (!this.current) return undefined;
    if (!entityExists) {
      this.clear();
      return undefined;
    }
    const next: Selection = { ...this.current, snapshotId };
    delete next.revisionId;
    delete next.projection;
    delete next.anchor;
    this.set(next);
    return next;
  }
}
