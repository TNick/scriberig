/** @file Projects one bounded geospatial feature page into an accessible SVG map. */
import { useMemo, useState, type ReactElement } from "react";
import type { GeoCoordinate, GeoFeatureView, GeoGeometry } from "@scriberig/protocol";
import { screenToLayerCoordinate } from "./geo-map-coordinates";

/** Represents finite map-plane ordinates used for SVG rendering. */
interface Point {
  /** Projected x coordinate in layer CRS units. */
  x: number;
  /** Projected y coordinate in layer CRS units. */
  y: number;
}

/** Represents the fitted bounds of the current feature page. */
interface Bounds {
  /** Minimum projected x coordinate. */
  minX: number;
  /** Minimum projected y coordinate. */
  minY: number;
  /** Maximum projected x coordinate. */
  maxX: number;
  /** Maximum projected y coordinate. */
  maxY: number;
}

/** Renders a bounded, provider-neutral feature page as selectable vector geometry.
 * @param props Exact loaded feature page and map selection callback.
 * @returns Accessible SVG projection or a safe empty-state message.
 */
export function GeoMapCanvas({
  features,
  crsLabel,
  onSelectFeature,
  onMapCoordinate,
}: {
  /** Features from one bounded exact-snapshot query page. */
  features: readonly GeoFeatureView[];
  /** Explicit coordinate reference label for the displayed values. */
  crsLabel: string;
  /** Publishes a selected exact semantic feature identity. */
  onSelectFeature?: ((feature: GeoFeatureView) => void) | undefined;
  /** Publishes a map position converted to the displayed layer CRS. */
  onMapCoordinate?: ((coordinate: GeoCoordinate) => void) | undefined;
}): ReactElement {
  const positions = useMemo(
    () => features.flatMap((feature) => (feature.geometry ? coordinates(feature.geometry) : [])),
    [features],
  );
  const [viewport, setViewport] = useState({ scale: 1, panX: 0, panY: 0 });
  const bounds = collectBounds(positions);
  if (bounds === undefined) {
    return (
      <figure aria-label="Geospatial map">
        <figcaption>Feature page in {crsLabel}</figcaption>
        <p role="status">No renderable geometries are available in this bounded page.</p>
      </figure>
    );
  }
  const width = Math.max(bounds.maxX - bounds.minX, 1);
  const height = Math.max(bounds.maxY - bounds.minY, 1);
  const padding = Math.max(width, height) * 0.08;
  const viewWidth = (width + padding * 2) / viewport.scale;
  const viewHeight = (height + padding * 2) / viewport.scale;
  const centerX = (bounds.minX + bounds.maxX) / 2 + viewport.panX * width;
  const centerY = (bounds.minY + bounds.maxY) / 2 + viewport.panY * height;
  const viewBounds = {
    minX: centerX - viewWidth / 2,
    minSvgY: -(centerY + viewHeight / 2),
    width: viewWidth,
    height: viewHeight,
  };
  const viewBox = [viewBounds.minX, viewBounds.minSvgY, viewBounds.width, viewBounds.height].join(
    " ",
  );
  return (
    <figure aria-label="Geospatial map">
      <figcaption>
        Bounded feature page in {crsLabel}; source coordinates are preserved.
        {onMapCoordinate ? ` Click empty map space to set a point in ${crsLabel}.` : ""}
      </figcaption>
      <nav aria-label="Map viewport controls">
        <button
          type="button"
          aria-label="Zoom out map"
          onClick={() =>
            setViewport((current) => ({ ...current, scale: Math.max(0.25, current.scale / 1.25) }))
          }
        >
          −
        </button>
        <button
          type="button"
          aria-label="Zoom in map"
          onClick={() =>
            setViewport((current) => ({ ...current, scale: Math.min(8, current.scale * 1.25) }))
          }
        >
          +
        </button>
        <button
          type="button"
          aria-label="Pan map left"
          onClick={() =>
            setViewport((current) => ({ ...current, panX: Math.max(-3, current.panX - 0.2) }))
          }
        >
          ←
        </button>
        <button
          type="button"
          aria-label="Pan map right"
          onClick={() =>
            setViewport((current) => ({ ...current, panX: Math.min(3, current.panX + 0.2) }))
          }
        >
          →
        </button>
        <button
          type="button"
          aria-label="Pan map up"
          onClick={() =>
            setViewport((current) => ({ ...current, panY: Math.min(3, current.panY + 0.2) }))
          }
        >
          ↑
        </button>
        <button
          type="button"
          aria-label="Pan map down"
          onClick={() =>
            setViewport((current) => ({ ...current, panY: Math.max(-3, current.panY - 0.2) }))
          }
        >
          ↓
        </button>
        <button type="button" onClick={() => setViewport({ scale: 1, panX: 0, panY: 0 })}>
          Reset map view
        </button>
      </nav>
      <svg
        role="group"
        aria-label="Semantic geospatial features"
        viewBox={viewBox}
        width="100%"
        height="360"
        preserveAspectRatio="xMidYMid meet"
        onClick={(event) => {
          if (event.target !== event.currentTarget || onMapCoordinate === undefined) return;
          const rectangle = event.currentTarget.getBoundingClientRect();
          onMapCoordinate(
            screenToLayerCoordinate(event.clientX, event.clientY, rectangle, viewBounds),
          );
        }}
      >
        {features.map((feature) =>
          feature.geometry ? (
            <FeatureMark
              key={feature.id}
              feature={feature}
              onSelect={() => onSelectFeature?.(feature)}
            />
          ) : null,
        )}
      </svg>
    </figure>
  );
}

/** Projects one feature geometry into selectable SVG primitives.
 * @param props Semantic feature and activation callback.
 * @returns SVG marks carrying the stable feature identity.
 */
function FeatureMark({
  feature,
  onSelect,
}: {
  feature: GeoFeatureView;
  onSelect: () => void;
}): ReactElement {
  const geometry = feature.geometry;
  if (geometry === undefined) return <g />;
  const label = `Feature ${feature.nativeId ?? feature.id} (${feature.id})`;
  return (
    <g
      data-entity-id={feature.id}
      role="button"
      tabIndex={0}
      aria-label={label}
      onClick={onSelect}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") onSelect();
      }}
    >
      {geometryMarks(geometry, feature.id)}
    </g>
  );
}

/** Returns all coordinates nested in one geometry.
 * @param geometry Typed semantic geometry.
 * @returns Exact coordinates for presentation projection.
 */
function coordinates(geometry: GeoGeometry): GeoCoordinate[] {
  switch (geometry.kind) {
    case "point":
      return [geometry.coordinates];
    case "line_string":
      return geometry.coordinates;
    case "polygon":
      return geometry.coordinates.flat();
    case "collection":
      return geometry.coordinates.flatMap(coordinates);
  }
}

/** Projects a recursive geometry as one or more SVG path strings.
 * @param geometry Typed semantic geometry.
 * @param featureId Stable feature identity used to key generated SVG elements.
 * @returns SVG path commands in the view's local CRS coordinate plane.
 */
function geometryMarks(geometry: GeoGeometry, featureId: string): ReactElement[] {
  if (geometry.kind === "collection") {
    return geometry.coordinates.flatMap((part, index) =>
      geometryMarks(part, `${featureId}-${index}`),
    );
  }
  if (geometry.kind === "point") {
    try {
      const point = pointOf(geometry.coordinates);
      return [
        <circle
          key={featureId}
          cx={point.x}
          cy={-point.y}
          r={3}
          fill="currentColor"
          data-feature-part={featureId}
        />,
      ];
    } catch {
      return [];
    }
  }
  const rings = geometry.kind === "line_string" ? [geometry.coordinates] : geometry.coordinates;
  return rings.flatMap((ring, index) => {
    try {
      const points = ring.map(pointOf);
      const first = points[0];
      if (first === undefined) return [];
      const path = `M ${first.x} ${-first.y} ${points
        .slice(1)
        .map((point) => `L ${point.x} ${-point.y}`)
        .join(" ")}${geometry.kind === "polygon" ? " Z" : ""}`;
      return [
        <path
          key={`${featureId}-${index}`}
          d={path}
          fill={geometry.kind === "polygon" ? "currentColor" : "none"}
          fillOpacity={0.12}
          stroke="currentColor"
          strokeWidth={1.5}
          vectorEffect="non-scaling-stroke"
        />,
      ];
    } catch {
      return [];
    }
  });
}

/** Converts exact decimal coordinate text for SVG presentation only.
 * @param coordinate Semantic coordinate in the layer CRS.
 * @returns Finite numeric point in the display plane.
 * @throws When a coordinate cannot be projected to a finite SVG value.
 */
function pointOf(coordinate: GeoCoordinate): Point {
  const x = Number(coordinate.x);
  const y = Number(coordinate.y);
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    throw new TypeError("feature coordinate cannot be projected into the map viewport");
  }
  return { x, y };
}

/** Calculates finite bounds for a bounded feature page.
 * @param values Coordinates in the selected layer CRS.
 * @returns Map bounds, or undefined when every coordinate is invalid.
 */
function collectBounds(values: readonly GeoCoordinate[]): Bounds | undefined {
  const points = values.flatMap((value) => {
    try {
      return [pointOf(value)];
    } catch {
      return [];
    }
  });
  if (points.length === 0) return undefined;
  return points.reduce<Bounds>(
    (bounds, point) => ({
      minX: Math.min(bounds.minX, point.x),
      minY: Math.min(bounds.minY, point.y),
      maxX: Math.max(bounds.maxX, point.x),
      maxY: Math.max(bounds.maxY, point.y),
    }),
    { minX: points[0]!.x, minY: points[0]!.y, maxX: points[0]!.x, maxY: points[0]!.y },
  );
}
