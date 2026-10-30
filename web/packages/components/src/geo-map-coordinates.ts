/** @file Converts map pointer positions into layer coordinate reference values. */
import type { GeoCoordinate } from "@scriberig/protocol";

/** Describes the screen rectangle occupied by the SVG map viewport. */
export interface GeoMapScreenRect {
  /** Left edge of the SVG in CSS pixels. */
  left: number;
  /** Top edge of the SVG in CSS pixels. */
  top: number;
  /** Width of the SVG in CSS pixels. */
  width: number;
  /** Height of the SVG in CSS pixels. */
  height: number;
}

/** Describes the layer-coordinate viewBox rendered by the map SVG. */
export interface GeoMapViewBox {
  /** Leftmost x ordinate in layer CRS units. */
  minX: number;
  /** Topmost SVG y ordinate, expressed as negated layer y. */
  minSvgY: number;
  /** Width of the rendered coordinate extent. */
  width: number;
  /** Height of the rendered coordinate extent. */
  height: number;
}

/** Converts a screen pointer position to exact text ordinates in the layer CRS.
 *
 * The calculation mirrors SVG `xMidYMid meet` letterboxing and the map's
 * presentation-only y-axis inversion. It does not reproject between CRSs.
 * @param clientX Pointer x position in CSS pixels.
 * @param clientY Pointer y position in CSS pixels.
 * @param screen SVG rectangle in CSS pixels.
 * @param viewBox Exact numeric bounds currently rendered by the map.
 * @returns Layer CRS x/y ordinates suitable for a semantic geometry request.
 * @throws When pointer or viewport geometry is non-finite or degenerate.
 */
export function screenToLayerCoordinate(
  clientX: number,
  clientY: number,
  screen: GeoMapScreenRect,
  viewBox: GeoMapViewBox,
): GeoCoordinate {
  const values = [
    clientX,
    clientY,
    screen.left,
    screen.top,
    screen.width,
    screen.height,
    viewBox.minX,
    viewBox.minSvgY,
    viewBox.width,
    viewBox.height,
  ];
  if (!values.every(Number.isFinite) || screen.width <= 0 || screen.height <= 0) {
    throw new RangeError("map pointer or viewport bounds are invalid");
  }
  if (viewBox.width <= 0 || viewBox.height <= 0) {
    throw new RangeError("map coordinate viewBox must have positive dimensions");
  }

  const scale = Math.min(screen.width / viewBox.width, screen.height / viewBox.height);
  const drawnWidth = viewBox.width * scale;
  const drawnHeight = viewBox.height * scale;
  const letterboxX = (screen.width - drawnWidth) / 2;
  const letterboxY = (screen.height - drawnHeight) / 2;
  const svgX = (clientX - screen.left - letterboxX) / scale + viewBox.minX;
  const svgY = (clientY - screen.top - letterboxY) / scale + viewBox.minSvgY;
  const x = svgX;
  const y = -svgY;
  if (!Number.isFinite(x) || !Number.isFinite(y)) {
    throw new RangeError("map pointer cannot be represented in layer coordinates");
  }
  return { x: String(x), y: String(y) };
}
