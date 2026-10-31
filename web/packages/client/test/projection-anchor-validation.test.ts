/** @file Verifies bounded transport validation for domain projection anchors. */
import { describe, expect, it } from "vitest";
import { validateProjectionAnchor } from "../src";

describe("projection anchor validation", () => {
  it("accepts bounded CAD coordinates and rejects oversized identifiers", () => {
    expect(() =>
      validateProjectionAnchor({
        kind: "cadBounds",
        drawingId: "ent_drawing" as never,
        space: "model",
        minX: "-1.25",
        minY: "0",
        maxX: "4.75",
        maxY: "8",
      }),
    ).not.toThrow();
    expect(() =>
      validateProjectionAnchor({
        kind: "cadBounds",
        drawingId: "d".repeat(129) as never,
        space: "model",
        minX: "0",
        minY: "0",
        maxX: "1",
        maxY: "1",
      }),
    ).toThrow(RangeError);
  });
});
