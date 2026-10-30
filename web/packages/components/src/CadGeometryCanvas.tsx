/** @file Projects a bounded page of semantic CAD lines and polylines into SVG. */
import { useEffect, useState, type ReactElement, type KeyboardEvent } from "react";
import type { CadCoordinate, CadEntitySummary } from "@scriberig/protocol";
import type { CadProjectionBounds } from "@scriberig/editor-core";
import {
  compareExactDecimal,
  exactAdd,
  exactMaximum,
  exactMinimum,
  exactSubtract,
} from "./cad-geometry-decimal";

/** Describes one exact semantic CAD target and its projection-space bounds. */
export interface CadProjectedEntitySelection {
  /** Semantic entity activated by the user. */
  entity: CadEntitySummary;
  /** Exact decimal bounds derived from authored geometry coordinates. */
  bounds: CadProjectionBounds;
}

interface ProjectedEntity extends CadProjectedEntitySelection {
  /** Numeric coordinates used only to draw the SVG preview. */
  points: Array<{ x: number; y: number }>;
  /** Projected CAD geometry kind. */
  kind: "line" | "polyline" | "polygon" | "circle" | "arcBounds" | "text" | "dimension" | "block";
  /** Numeric bounds used to fit the geometry into the presentation viewport. */
  numericBounds: NumericBounds;
  /** Radius for circle geometry and arc extent placeholders. */
  radius?: number;
  /** Authored text displayed at its insertion point. */
  label?: string;
}

interface NumericBounds {
  /** Minimum x in the displayed page. */
  minX: number;
  /** Minimum y in the displayed page. */
  minY: number;
  /** Maximum x in the displayed page. */
  maxX: number;
  /** Maximum y in the displayed page. */
  maxY: number;
}

/** Projects a bounded CAD entity page as an accessible vector preview.
 * @param props Entities, visible layers, drawing identity, and selection callback.
 * @returns SVG for supported lines and polylines, or a safe unsupported-data message.
 */
export function CadGeometryCanvas({
  entities,
  visibleLayerIds,
  onSelectEntity,
}: {
  /** Exact semantic entities from the current bounded page. */
  entities: readonly CadEntitySummary[];
  /** Semantic layers currently enabled in the drawing projection. */
  visibleLayerIds: ReadonlySet<string>;
  /** Publishes the exact entity and its authored coordinate bounds. */
  onSelectEntity?: ((selection: CadProjectedEntitySelection) => void) | undefined;
}): ReactElement {
  const projected = entities.flatMap((entity) => {
    if (!visibleLayerIds.has(entity.layer)) return [];
    const projection = projectEntity(entity);
    return projection === undefined ? [] : [projection];
  });
  const pageBounds = collectBounds(projected);
  const pageKey = projected.map((item) => item.entity.id).join("|");
  const [viewport, setViewport] = useState({ scale: 1, panX: 0, panY: 0 });
  useEffect(() => setViewport({ scale: 1, panX: 0, panY: 0 }), [pageKey]);
  if (projected.length === 0 || pageBounds === undefined) {
    return (
      <figure aria-label="CAD vector preview">
        <figcaption>Vector preview for the current bounded entity page</figcaption>
        <p role="status">No visible supported geometry is available to preview.</p>
      </figure>
    );
  }
  const width = Math.max(pageBounds.maxX - pageBounds.minX, 1);
  const height = Math.max(pageBounds.maxY - pageBounds.minY, 1);
  const padding = Math.max(width, height) * 0.05;
  const viewWidth = (width + padding * 2) / viewport.scale;
  const viewHeight = (height + padding * 2) / viewport.scale;
  const centerX = (pageBounds.minX + pageBounds.maxX) / 2 + viewport.panX * width;
  const centerY = (pageBounds.minY + pageBounds.maxY) / 2 + viewport.panY * height;
  const markerRadius = Math.max(width, height) * 0.012;
  const viewBox = [centerX - viewWidth / 2, -centerY - viewHeight / 2, viewWidth, viewHeight].join(
    " ",
  );
  return (
    <figure aria-label="CAD vector preview">
      <figcaption>Vector preview for the current bounded entity page</figcaption>
      <nav aria-label="CAD preview viewport controls">
        <button
          type="button"
          aria-label="Zoom out"
          onClick={() =>
            setViewport((current) => ({ ...current, scale: Math.max(0.25, current.scale / 1.25) }))
          }
        >
          −
        </button>
        <button
          type="button"
          aria-label="Zoom in"
          onClick={() =>
            setViewport((current) => ({ ...current, scale: Math.min(8, current.scale * 1.25) }))
          }
        >
          +
        </button>
        <button
          type="button"
          aria-label="Pan left"
          onClick={() =>
            setViewport((current) => ({ ...current, panX: Math.max(-3, current.panX - 0.2) }))
          }
        >
          ←
        </button>
        <button
          type="button"
          aria-label="Pan right"
          onClick={() =>
            setViewport((current) => ({ ...current, panX: Math.min(3, current.panX + 0.2) }))
          }
        >
          →
        </button>
        <button
          type="button"
          aria-label="Pan up"
          onClick={() =>
            setViewport((current) => ({ ...current, panY: Math.min(3, current.panY + 0.2) }))
          }
        >
          ↑
        </button>
        <button
          type="button"
          aria-label="Pan down"
          onClick={() =>
            setViewport((current) => ({ ...current, panY: Math.max(-3, current.panY - 0.2) }))
          }
        >
          ↓
        </button>
        <button type="button" onClick={() => setViewport({ scale: 1, panX: 0, panY: 0 })}>
          Reset view
        </button>
      </nav>
      <svg
        role="group"
        aria-label="Semantic CAD entities"
        viewBox={viewBox}
        width="100%"
        height="400"
        preserveAspectRatio="xMidYMid meet"
      >
        {projected.map((item) => (
          <EntityMark
            key={item.entity.id}
            projected={item}
            markerRadius={markerRadius}
            onSelect={onSelectEntity}
          />
        ))}
      </svg>
    </figure>
  );
}

/** Renders one keyboard-selectable projected CAD entity.
 * @param props Projected shape and semantic selection callback.
 * @returns A focusable SVG group with a stable entity identity.
 */
function EntityMark({
  projected,
  markerRadius,
  onSelect,
}: {
  /** Supported geometry and exact source coordinate bounds. */
  projected: ProjectedEntity;
  /** Marker radius for point-like semantic entities, in drawing coordinates. */
  markerRadius: number;
  /** Publishes the selected entity and exact bounds. */
  onSelect?: ((selection: CadProjectedEntitySelection) => void) | undefined;
}): ReactElement {
  const activate = (): void => onSelect?.(projected);
  const onKeyDown = (event: KeyboardEvent<SVGGElement>): void => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      activate();
    }
  };
  const first = projected.points[0];
  const last = projected.points.at(-1);
  if (first === undefined || last === undefined) return <g />;
  return (
    <g
      role="button"
      tabIndex={0}
      data-entity-id={projected.entity.id}
      data-min-x={projected.bounds.minX}
      data-min-y={projected.bounds.minY}
      data-max-x={projected.bounds.maxX}
      data-max-y={projected.bounds.maxY}
      aria-label={entityAccessibleLabel(projected)}
      onClick={activate}
      onKeyDown={onKeyDown}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      vectorEffect="non-scaling-stroke"
    >
      {projected.kind === "line" ? (
        <line x1={first.x} y1={-first.y} x2={last.x} y2={-last.y} />
      ) : projected.kind === "circle" || projected.kind === "arcBounds" ? (
        <>
          {projected.kind === "arcBounds" ? (
            <title>Arc extent placeholder, not rendered arc geometry</title>
          ) : null}
          <circle
            cx={first.x}
            cy={-first.y}
            r={projected.radius}
            strokeDasharray={projected.kind === "arcBounds" ? "4 3" : undefined}
          />
        </>
      ) : projected.kind === "text" ? (
        <text x={first.x} y={-first.y} fontSize={markerRadius * 2} fill="currentColor">
          {projected.label}
        </text>
      ) : projected.kind === "block" ? (
        <circle cx={first.x} cy={-first.y} r={markerRadius} />
      ) : projected.kind === "polygon" ? (
        <polygon points={projected.points.map((point) => `${point.x},${-point.y}`).join(" ")} />
      ) : (
        <polyline points={projected.points.map((point) => `${point.x},${-point.y}`).join(" ")} />
      )}
      {projected.kind !== "text" && projected.kind !== "block" ? (
        projected.kind === "line" ? (
          <line
            x1={first.x}
            y1={-first.y}
            x2={last.x}
            y2={-last.y}
            stroke="transparent"
            strokeWidth="12"
            vectorEffect="non-scaling-stroke"
          />
        ) : projected.kind === "circle" || projected.kind === "arcBounds" ? (
          <circle
            cx={first.x}
            cy={-first.y}
            r={projected.radius}
            stroke="transparent"
            strokeWidth="12"
            vectorEffect="non-scaling-stroke"
          />
        ) : projected.kind === "polygon" ? (
          <polygon
            points={projected.points.map((point) => `${point.x},${-point.y}`).join(" ")}
            stroke="transparent"
            strokeWidth="12"
            vectorEffect="non-scaling-stroke"
          />
        ) : (
          <polyline
            points={projected.points.map((point) => `${point.x},${-point.y}`).join(" ")}
            stroke="transparent"
            strokeWidth="12"
            vectorEffect="non-scaling-stroke"
          />
        )
      ) : null}
    </g>
  );
}

/** Describes the selected shape or bounding placeholder accessibly.
 * @param projected Projected CAD entity.
 * @returns Accessible label that identifies approximate arc bounds honestly.
 */
function entityAccessibleLabel(projected: ProjectedEntity): string {
  return projected.kind === "arcBounds"
    ? `Select CAD arc bounds for entity ${projected.entity.id}`
    : `Select CAD entity ${projected.entity.id}`;
}

/** Converts one line or polyline into a safe two-dimensional preview.
 * @param entity Exact semantic CAD entity.
 * @returns SVG geometry and exact coordinate bounds when representable.
 */
function projectEntity(entity: CadEntitySummary): ProjectedEntity | undefined {
  const geometry = entity.geometry;
  let coordinates: CadCoordinate[];
  let kind: ProjectedEntity["kind"];
  let radiusText: string | undefined;
  let label: string | undefined;
  if ("Line" in geometry) {
    coordinates = [geometry.Line.start, geometry.Line.end];
    kind = "line";
  } else if ("Polyline" in geometry) {
    coordinates = geometry.Polyline.vertices;
    kind = geometry.Polyline.closed ? "polygon" : "polyline";
  } else if ("Circle" in geometry || "Arc" in geometry) {
    const circle = "Circle" in geometry ? geometry.Circle : geometry.Arc;
    coordinates = [circle.center];
    radiusText = circle.radius;
    kind = "Circle" in geometry ? "circle" : "arcBounds";
    if ((compareExactDecimal(radiusText, "0") ?? -1) < 0) return undefined;
  } else if ("Text" in geometry) {
    coordinates = [geometry.Text.position];
    label = geometry.Text.value;
    kind = "text";
  } else if ("Dimension" in geometry) {
    coordinates = geometry.Dimension.definition_points;
    kind = "dimension";
  } else {
    coordinates = [geometry.BlockReference.insertion];
    kind = "block";
  }
  if (
    coordinates.length === 0 ||
    (coordinates.length < 2 && radiusText === undefined && label === undefined && kind !== "block")
  ) {
    return undefined;
  }
  const points = coordinates.map(({ x, y }) => ({ x: Number(x), y: Number(y) }));
  if (points.some((point) => !Number.isFinite(point.x) || !Number.isFinite(point.y))) {
    return undefined;
  }
  const bounds = geometryBounds(coordinates, radiusText);
  if (bounds === undefined) return undefined;
  const numericBounds: NumericBounds = {
    minX: Number(bounds.minX),
    minY: Number(bounds.minY),
    maxX: Number(bounds.maxX),
    maxY: Number(bounds.maxY),
  };
  if (Object.values(numericBounds).some((value) => !Number.isFinite(value))) return undefined;
  return {
    entity,
    bounds,
    numericBounds,
    points,
    kind,
    ...(radiusText === undefined ? {} : { radius: Number(radiusText) }),
    ...(label === undefined ? {} : { label }),
  };
}

/** Computes exact geometry bounds for point, line, polyline, circle, or arc input.
 * @param coordinates Exact geometry coordinate points.
 * @param radius Optional exact radius for circles and arc extents.
 * @returns Exact decimal bounds or undefined for unsupported coordinate text.
 */
function geometryBounds(
  coordinates: readonly CadCoordinate[],
  radius?: string,
): CadProjectionBounds | undefined {
  const xs = coordinates.map((point) => point.x);
  const ys = coordinates.map((point) => point.y);
  if (radius === undefined) {
    const minX = exactMinimum(xs);
    const minY = exactMinimum(ys);
    const maxX = exactMaximum(xs);
    const maxY = exactMaximum(ys);
    return minX === undefined || minY === undefined || maxX === undefined || maxY === undefined
      ? undefined
      : { minX, minY, maxX, maxY };
  }
  if (coordinates.length !== 1) return undefined;
  const center = coordinates[0];
  if (center === undefined) return undefined;
  const minX = exactSubtract(center.x, radius);
  const minY = exactSubtract(center.y, radius);
  const maxX = exactAdd(center.x, radius);
  const maxY = exactAdd(center.y, radius);
  return minX === undefined || minY === undefined || maxX === undefined || maxY === undefined
    ? undefined
    : { minX, minY, maxX, maxY };
}

/** Combines preview coordinates into one page view box.
 * @param entities Supported visible semantic entities.
 * @returns Finite page bounds, or undefined when no coordinates exist.
 */
function collectBounds(entities: readonly ProjectedEntity[]): NumericBounds | undefined {
  if (entities.length === 0) return undefined;
  return {
    minX: Math.min(...entities.map((entity) => entity.numericBounds.minX)),
    minY: Math.min(...entities.map((entity) => entity.numericBounds.minY)),
    maxX: Math.max(...entities.map((entity) => entity.numericBounds.maxX)),
    maxY: Math.max(...entities.map((entity) => entity.numericBounds.maxY)),
  };
}
