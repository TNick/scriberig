/** @file Validates projection anchors before they cross the client transport. */
import type { ProjectionAnchor } from "@scriberig/protocol";

/** Checks whether a coordinate is a safe integer at or above its minimum.
 * @param value Coordinate supplied by a client.
 * @param minimum Smallest valid coordinate.
 * @returns Whether the coordinate is valid.
 */
function isSafeIntegerAtLeast(value: number, minimum: number): boolean {
  return Number.isSafeInteger(value) && value >= minimum;
}

/** Returns the UTF-8 byte length of one descriptor string.
 * @param value Descriptor text.
 * @returns Encoded byte count used by the wire contract.
 */
function utf8Length(value: string): number {
  return new TextEncoder().encode(value).length;
}

/** Checks that descriptor text is nonempty and within a UTF-8 byte bound.
 * @param value Descriptor text.
 * @param maximum Maximum allowed encoded bytes.
 * @returns Whether the descriptor is bounded and nonempty.
 */
function isBoundedText(value: string, maximum: number): boolean {
  const length = utf8Length(value);
  return length > 0 && length <= maximum;
}

/** Validates coordinate ordering and byte bounds for a transported projection anchor.
 * @param anchor Typed view context carried beside semantic entity and snapshot identity.
 * @throws When coordinates are invalid or a descriptor string exceeds the wire limit.
 */
export function validateProjectionAnchor(anchor: ProjectionAnchor): void {
  switch (anchor.kind) {
    case "textRange":
      if (
        !isSafeIntegerAtLeast(anchor.start, 0) ||
        !isSafeIntegerAtLeast(anchor.end, anchor.start)
      ) {
        throw new RangeError("text anchor range is invalid");
      }
      return;
    case "pageRegion":
      if (
        !isSafeIntegerAtLeast(anchor.page, 1) ||
        !isSafeIntegerAtLeast(anchor.x, 0) ||
        !isSafeIntegerAtLeast(anchor.y, 0) ||
        !isSafeIntegerAtLeast(anchor.width, 1) ||
        !isSafeIntegerAtLeast(anchor.height, 1)
      ) {
        throw new RangeError("page region anchor coordinates are invalid");
      }
      return;
    case "spreadsheetCell":
      if (
        !isBoundedText(anchor.worksheetId, 128) ||
        !isSafeIntegerAtLeast(anchor.row, 1) ||
        !isSafeIntegerAtLeast(anchor.column, 1)
      ) {
        throw new RangeError("spreadsheet cell anchor coordinates are invalid");
      }
      return;
    case "spreadsheetRange":
      if (
        !isBoundedText(anchor.worksheetId, 128) ||
        !isSafeIntegerAtLeast(anchor.firstRow, 1) ||
        !isSafeIntegerAtLeast(anchor.firstColumn, 1) ||
        !isSafeIntegerAtLeast(anchor.lastRow, anchor.firstRow) ||
        !isSafeIntegerAtLeast(anchor.lastColumn, anchor.firstColumn)
      ) {
        throw new RangeError("spreadsheet range anchor coordinates are invalid");
      }
      return;
    case "geometryBounds":
      if (
        !isBoundedText(anchor.crs, 256) ||
        !isBoundedText(anchor.minX, 128) ||
        !isBoundedText(anchor.minY, 128) ||
        !isBoundedText(anchor.maxX, 128) ||
        !isBoundedText(anchor.maxY, 128)
      ) {
        throw new RangeError("geometry anchor values exceed descriptor bounds");
      }
      return;
    case "cadBounds":
      if (
        !isBoundedText(anchor.drawingId, 128) ||
        !isBoundedText(anchor.space, 128) ||
        !isBoundedText(anchor.minX, 128) ||
        !isBoundedText(anchor.minY, 128) ||
        !isBoundedText(anchor.maxX, 128) ||
        !isBoundedText(anchor.maxY, 128)
      ) {
        throw new RangeError("CAD anchor values exceed descriptor bounds");
      }
      return;
    case "opaque":
      if (
        !isBoundedText(anchor.adapter, 128) ||
        !isBoundedText(anchor.anchorKind, 128) ||
        utf8Length(anchor.value) > 4096
      ) {
        throw new RangeError("opaque anchor values exceed descriptor bounds");
      }
  }
}
