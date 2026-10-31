/** @file Verifies bounded geospatial reads and exact-base candidate operations. */
import { describe, expect, it, vi } from "vitest";
import type { ScriberigClient } from "@scriberig/client";
import type {
  GeoCatalogResponse,
  GeoFeatureQueryResponse,
  GeoFeaturesResponse,
} from "@scriberig/protocol";
import { GeoPageController } from "../src/geo-page-controller";
import sharedGeoTarget from "../../../../tests/fixtures/rich-artifact/geo-feature-target.json";

const layer = {
  id: sharedGeoTarget.layer,
  dataset: "dataset-1",
  name: "parcels",
  fields: [{ name: sharedGeoTarget.field, fieldType: "number" as const, nullable: false }],
  crs: { authority: "EPSG", code: "4326" },
  geometryType: "point" as const,
  featureCount: 100_000,
  extent: ["0", "0", "100", "100"] as [string, string, string, string],
  editable: true,
  diagnostics: [],
};

const catalog: GeoCatalogResponse = {
  project: sharedGeoTarget.project,
  snapshot: sharedGeoTarget.snapshot,
  dataset: "dataset-1",
  sourceDigest: "aa".repeat(32),
  layers: [layer],
  total: 1,
  hasMore: false,
};

const featurePage: GeoFeaturesResponse = {
  project: sharedGeoTarget.project,
  snapshot: sharedGeoTarget.snapshot,
  layer: sharedGeoTarget.layer,
  features: [
    {
      id: sharedGeoTarget.feature,
      nativeId: sharedGeoTarget.nativeId,
      geometry: { kind: "point", coordinates: { x: "5.000", y: "7.000" } },
      attributes: { parcel_id: { kind: "number", value: "123.50" } },
    },
  ],
  hasMore: true,
};

const queryPage: GeoFeatureQueryResponse = {
  ...featurePage,
  sourceRevision: "revision-1",
  sourceDigest: "aa".repeat(32),
  provider: "geopackage",
  layerCrs: layer.crs,
  strategy: "spatial_index",
  diagnostics: [],
};

/** Creates a controller over deterministic GeoPackage responses. */
function fixture(editable = true) {
  const client = {
    inspectGeoPackage: vi.fn().mockResolvedValue(
      editable
        ? catalog
        : {
            ...catalog,
            layers: [{ ...layer, editable: false, diagnostics: ["provider is read only"] }],
          },
    ),
    geoFeatures: vi.fn().mockResolvedValue(featurePage),
    queryGeoFeatures: vi.fn().mockResolvedValue(queryPage),
    proposeGeoAttributeEdit: vi.fn().mockResolvedValue({
      candidate: "candidate-1",
      project: sharedGeoTarget.project,
      baseSnapshot: sharedGeoTarget.snapshot,
      layer: sharedGeoTarget.layer,
      feature: sharedGeoTarget.feature,
      field: sharedGeoTarget.field,
      path: "parcels.gpkg",
    }),
    proposeGeoGeometryEdit: vi.fn().mockResolvedValue({
      candidate: "candidate-2",
      project: sharedGeoTarget.project,
      baseSnapshot: sharedGeoTarget.snapshot,
      layer: sharedGeoTarget.layer,
      feature: sharedGeoTarget.feature,
      path: "parcels.gpkg",
    }),
  };
  return {
    client,
    controller: new GeoPageController(
      client as unknown as Pick<
        ScriberigClient,
        | "inspectGeoPackage"
        | "geoFeatures"
        | "queryGeoFeatures"
        | "proposeGeoAttributeEdit"
        | "proposeGeoGeometryEdit"
      >,
      sharedGeoTarget.project,
      "parcels.gpkg",
      sharedGeoTarget.snapshot,
      50,
    ),
  };
}

describe("GeoPageController", () => {
  it("targets the shared parcel feature from the cross-surface fixture", async () => {
    const { client, controller } = fixture();
    await controller.readCatalog();
    const page = await controller.readFeatures(sharedGeoTarget.layer);
    expect(page.features[0]?.nativeId).toBe(sharedGeoTarget.nativeId);
    await controller.proposeAttribute(sharedGeoTarget.layer, sharedGeoTarget.feature, "parcel_id", {
      kind: "number",
      value: sharedGeoTarget.value,
    });
    expect(client.proposeGeoAttributeEdit).toHaveBeenCalledWith(
      expect.objectContaining({
        project: sharedGeoTarget.project,
        snapshot: sharedGeoTarget.snapshot,
        layer: sharedGeoTarget.layer,
        feature: sharedGeoTarget.feature,
        field: sharedGeoTarget.field,
        value: { kind: "number", value: sharedGeoTarget.value },
      }),
      undefined,
    );
  });

  it("queries explicit CRS bounds with bounded paging and exact identities", async () => {
    const { client, controller } = fixture();
    await controller.readCatalog();
    const result = await controller.queryEnvelope(sharedGeoTarget.layer, {
      minX: "1.000",
      minY: "2",
      maxX: "9",
      maxY: "10",
      crs: { authority: "EPSG", code: "4326" },
    });
    expect(result.features[0]?.id).toBe(sharedGeoTarget.feature);
    expect(client.queryGeoFeatures).toHaveBeenCalledWith(
      expect.objectContaining({
        snapshot: sharedGeoTarget.snapshot,
        offset: 0,
        limit: 50,
        predicate: {
          kind: "envelope_overlaps",
          bounds: {
            minX: "1.000",
            minY: "2",
            maxX: "9",
            maxY: "10",
            crs: { authority: "EPSG", code: "4326" },
          },
        },
      }),
      undefined,
    );
  });

  it("rejects reversed bounds, missing CRS, and mismatched response scope", async () => {
    const { controller, client } = fixture();
    await controller.readCatalog();
    await expect(
      controller.queryEnvelope(sharedGeoTarget.layer, {
        minX: "9",
        minY: "0",
        maxX: "1",
        maxY: "10",
        crs: { authority: "EPSG", code: "4326" },
      }),
    ).rejects.toThrow("minimum bounds");
    await expect(
      controller.queryEnvelope(sharedGeoTarget.layer, {
        minX: "0",
        minY: "0",
        maxX: "1",
        maxY: "1",
        crs: {},
      }),
    ).rejects.toThrow("explicit CRS");
    client.geoFeatures.mockResolvedValueOnce({ ...featurePage, snapshot: "stale" });
    await expect(controller.readFeatures(sharedGeoTarget.layer)).rejects.toThrow("identity");
  });

  it("creates attribute and geometry candidates only for loaded editable targets", async () => {
    const { client, controller } = fixture();
    await controller.readCatalog();
    await controller.readFeatures(sharedGeoTarget.layer);
    const attribute = await controller.proposeAttribute(
      sharedGeoTarget.layer,
      sharedGeoTarget.feature,
      sharedGeoTarget.field,
      {
        kind: "number",
        value: "11.5",
      },
    );
    const geometry = await controller.proposeGeometry(
      sharedGeoTarget.layer,
      sharedGeoTarget.feature,
      {
        kind: "point",
        coordinates: { x: "6", y: "8" },
      },
    );
    expect(attribute.candidate).toBe("candidate-1");
    expect(geometry.candidate).toBe("candidate-2");
    expect(client.proposeGeoAttributeEdit).toHaveBeenCalledOnce();
    expect(client.proposeGeoGeometryEdit).toHaveBeenCalledOnce();
    await expect(
      controller.proposeAttribute(sharedGeoTarget.layer, "other-feature", sharedGeoTarget.field, {
        kind: "number",
        value: "12",
      }),
    ).rejects.toThrow("not present");
  });

  it("keeps read-only provider layers from creating edit candidates", async () => {
    const { client, controller } = fixture(false);
    await controller.readCatalog();
    await controller.readFeatures(sharedGeoTarget.layer);
    await expect(
      controller.proposeAttribute(
        sharedGeoTarget.layer,
        sharedGeoTarget.feature,
        sharedGeoTarget.field,
        {
          kind: "number",
          value: "11.5",
        },
      ),
    ).rejects.toThrow("provider is read only");
    expect(client.proposeGeoAttributeEdit).not.toHaveBeenCalled();
  });
});
