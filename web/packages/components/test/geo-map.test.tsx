/** @file Verifies map projection markup and screen-to-layer coordinate conversion. */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { GeoMapCanvas } from "../src/GeoMapCanvas";
import { screenToLayerCoordinate } from "../src/geo-map-coordinates";

describe("geospatial map components", () => {
  it("projects bounded geospatial features with exact selection IDs and CRS context", () => {
    const html = renderToStaticMarkup(
      <GeoMapCanvas
        crsLabel="EPSG:4326"
        features={[
          {
            id: "feature-124",
            nativeId: "124",
            geometry: {
              kind: "point",
              coordinates: { x: "1.2500", y: "2.5000" },
            },
            attributes: { parcel_id: { kind: "number", value: "124" } },
          },
        ]}
      />,
    );
    expect(html).toContain("Bounded feature page in EPSG:4326");
    expect(html).toContain('data-entity-id="feature-124"');
    expect(html).toContain("Feature 124 (feature-124)");
    expect(html).toContain("Zoom in map");
    expect(html).toContain("Reset map view");
  });

  it("converts screen points through the current SVG viewport into layer CRS coordinates", () => {
    expect(
      screenToLayerCoordinate(
        150,
        100,
        { left: 100, top: 50, width: 200, height: 100 },
        { minX: 0, minSvgY: -10, width: 20, height: 10 },
      ),
    ).toEqual({ x: "5", y: "5" });
    expect(
      screenToLayerCoordinate(
        200,
        100,
        { left: 100, top: 50, width: 200, height: 100 },
        { minX: 0, minSvgY: -10, width: 10, height: 10 },
      ),
    ).toEqual({ x: "5", y: "5" });
    expect(() =>
      screenToLayerCoordinate(
        0,
        0,
        { left: 0, top: 0, width: 0, height: 100 },
        { minX: 0, minSvgY: -1, width: 1, height: 1 },
      ),
    ).toThrow("invalid");
  });
});
