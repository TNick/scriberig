/** @file Verifies non-editable geospatial layers disable product edit controls. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { GeoFeatureView, GeoLayerView } from "@scriberig/protocol";
import { FeatureEditor, LayerSummary } from "../src/views/GeoFeatureEditor";

const layer: GeoLayerView = {
  id: "layer-parcels",
  dataset: "dataset-parcels",
  name: "Parcels",
  fields: [{ name: "label", fieldType: "text", nullable: false }],
  featureCount: 1,
  editable: false,
  diagnostics: ["layer has package structures outside the safe geometry writer subset"],
};

const feature: GeoFeatureView = {
  id: "feature-124",
  nativeId: "124",
  attributes: { label: { kind: "text", value: "Parcel 124" } },
};

describe("GeoFeatureEditor read-only capability", () => {
  it("keeps both candidate actions disabled for a non-editable layer", () => {
    const html = renderToStaticMarkup(
      <>
        <LayerSummary layer={layer} />
        <FeatureEditor
          controller={undefined}
          layer={layer}
          feature={feature}
          crsLabel="EPSG:4326"
          field="label"
          value="Updated parcel"
          geometryText="{}"
          activeField={layer.fields[0]}
          onField={vi.fn()}
          onValue={vi.fn()}
          onGeometry={vi.fn()}
          onCandidate={vi.fn()}
          onError={vi.fn()}
        />
      </>,
    );

    expect(html.match(/disabled=""/g)).toHaveLength(5);
    expect(html).toContain("Read only");
    expect(html).toContain("layer has package structures outside the safe geometry writer subset");
    expect(html).toContain("Replacement geometry JSON (EPSG:4326)");
    expect(html).toContain("Propose attribute");
    expect(html).toContain("Propose geometry");
  });
});
