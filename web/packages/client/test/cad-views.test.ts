/** @file Verifies strict normalization of remote CAD geometry views. */
import { describe, expect, it } from "vitest";
import { normalizeCadEntitiesResponse } from "../src/cad-views";

/** Creates one valid exact-snapshot response around supplied geometry.
 * @param geometry Raw protocol geometry payload.
 * @returns CAD entity page response object.
 */
function responseWith(geometry: unknown): unknown {
  return {
    project: "project-1",
    snapshot: "snapshot-2",
    drawing: "drawing-3",
    entities: [
      {
        id: "entity-4",
        layer: "layer-5",
        space: "Model",
        geometry,
      },
    ],
    total: 1,
    has_more: false,
  };
}

describe("CAD view normalizers", () => {
  it("preserves typed exact geometry values after validating each field", () => {
    const normalized = normalizeCadEntitiesResponse(
      responseWith({
        Text: {
          position: { x: "1.25", y: "2", z: "0" },
          value: "Parcel 124",
          height: "0.25",
        },
      }),
    );
    expect(normalized.entities[0]?.geometry).toEqual({
      Text: {
        position: { x: "1.25", y: "2", z: "0" },
        value: "Parcel 124",
        height: "0.25",
      },
    });
  });

  it("rejects a geometry variant with missing or malformed semantic fields", () => {
    expect(() =>
      normalizeCadEntitiesResponse(responseWith({ Line: { start: {}, end: {} } })),
    ).toThrow("coordinate x");
    expect(() =>
      normalizeCadEntitiesResponse(
        responseWith({ Polyline: { vertices: [{ x: "0", y: "0", z: "0" }], closed: "false" } }),
      ),
    ).toThrow("polyline closed");
    expect(() =>
      normalizeCadEntitiesResponse(
        responseWith({
          BlockReference: { definition: "block-1", insertion: {}, scale: ["1"], rotation: "0" },
        }),
      ),
    ).toThrow("block scale");
  });

  it("rejects ambiguous externally tagged CAD drawing spaces", () => {
    const response = responseWith({ Circle: { center: { x: "0", y: "0", z: "0" }, radius: "1" } });
    const record = response as Record<string, unknown>;
    const entities = record.entities as Array<Record<string, unknown>>;
    entities[0]!.space = { Paper: "layout-1", Named: "model-copy" };
    expect(() => normalizeCadEntitiesResponse(response)).toThrow("CAD drawing space is invalid");
  });
});
