/** @file Verifies bounded semantic change summaries for rich-artifact review. */
import { describe, expect, it } from "vitest";
import {
  boundedSemanticPreview,
  summarizeCadLayerChange,
  summarizeCadLineChange,
  summarizeGeoAttributeChange,
  summarizeGeoGeometryChange,
  summarizeQgisLayerRename,
  summarizeQgisLayerOrder,
} from "../src/semantic-change-summary";

describe("rich-artifact semantic change summaries", () => {
  it("shows exact CAD endpoint and layer changes by stable entity identity", () => {
    expect(
      summarizeCadLineChange(
        "entity-line-1",
        {
          start: { x: "0.00", y: "1", z: "0" },
          end: { x: "2", y: "3", z: "0" },
        },
        {
          start: { x: "0.25", y: "1", z: "0" },
          end: { x: "2", y: "4.5", z: "0" },
        },
      ),
    ).toBe(
      "CAD entity entity-line-1, line endpoints: start (0.00, 1, 0) → (0.25, 1, 0); end (2, 3, 0) → (2, 4.5, 0)",
    );
    expect(summarizeCadLayerChange("entity-line-1", "PARCELS", "LABELS")).toBe(
      "CAD entity entity-line-1, layer: PARCELS → LABELS",
    );
  });

  it("shows typed GeoPackage attribute values by stable feature identity", () => {
    expect(
      summarizeGeoAttributeChange(
        {
          id: "feature-124",
          nativeId: "124",
          attributes: { area: { kind: "number", value: "10.5" } },
        },
        "area",
        { kind: "number", value: "12.75" },
      ),
    ).toBe("Feature 124, attribute area: number 10.5 → number 12.75");
  });

  it("shows bounded GeoPackage geometry changes as structured semantic values", () => {
    expect(
      summarizeGeoGeometryChange(
        {
          id: "feature-124",
          attributes: {},
          geometry: { kind: "point", coordinates: { x: "1.25", y: "2.5" } },
        },
        { kind: "point", coordinates: { x: "3", y: "4" } },
      ),
    ).toBe(
      'Feature feature-124, geometry: {"kind":"point","coordinates":{"x":"1.25","y":"2.5"}} → {"kind":"point","coordinates":{"x":"3","y":"4"}}',
    );
    expect(boundedSemanticPreview("🧭".repeat(140))).toBe("🧭".repeat(120) + "…");
  });

  it("shows QGIS layer-reference rename without merging linked layer identity", () => {
    expect(summarizeQgisLayerRename("reference-3", "Parcels", "Tax Parcels")).toBe(
      "QGIS layer reference reference-3 changes name from “Parcels” to “Tax Parcels”.",
    );
  });

  it("shows the complete QGIS layer order by authored names", () => {
    const first = { id: "ref-1", name: "Parcels", dataSource: "p.gpkg", visible: true };
    const second = { id: "ref-2", name: "Roads", dataSource: "r.gpkg", visible: true };
    expect(summarizeQgisLayerOrder([first, second], [second, first])).toBe(
      "QGIS project layer order: Parcels → Roads → Roads → Parcels.",
    );
  });
});
