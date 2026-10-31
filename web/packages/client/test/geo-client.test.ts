/** @file Verifies geospatial feature client normalization and bounds. */
import { describe, expect, it, vi } from "vitest";
import { SemanticClient } from "../src/semantic-client";
import type { SemanticTransport } from "../src/transport";

describe("geospatial semantic client", () => {
  it("normalizes bounded layer metadata without coercing extent values", async () => {
    const request = vi.fn().mockResolvedValue({
      project: "project-1",
      snapshot: "snapshot-2",
      dataset: "dataset-3",
      source_digest: "ab".repeat(32),
      layers: [
        {
          id: "layer-4",
          dataset: "dataset-3",
          name: "parcels",
          fields: [{ name: "parcel_id", field_type: "number", nullable: false }],
          crs: { authority: "EPSG", code: "4326", definition: "EPSG:4326" },
          geometry_type: "point",
          feature_count: 2,
          extent: ["1.000", "2.000", "3.000", "4.000"],
          editable: false,
          diagnostics: ["writer unavailable"],
        },
      ],
      total: 1,
      has_more: false,
    });
    const client = new SemanticClient({ request } as unknown as SemanticTransport);
    const catalog = await client.inspectGeoPackage({
      project: "project-1",
      path: "parcels.gpkg",
      snapshot: "snapshot-2",
      offset: 0,
      limit: 50,
    });
    expect(request).toHaveBeenCalledWith(
      "geo_catalog",
      expect.objectContaining({ path: "parcels.gpkg", snapshot: "snapshot-2", limit: 50 }),
      undefined,
    );
    expect(catalog.layers[0]).toMatchObject({
      id: "layer-4",
      featureCount: 2,
      geometryType: "point",
      fields: [{ fieldType: "number", nullable: false }],
      extent: ["1.000", "2.000", "3.000", "4.000"],
      crs: { code: "4326" },
      editable: false,
    });
  });

  it("normalizes bounded exact-coordinate feature pages", async () => {
    const request = vi.fn().mockResolvedValue({
      project: "project-1",
      snapshot: "snapshot-2",
      layer: "layer-3",
      features: [
        {
          id: "feature-4",
          native_id: "124",
          geometry: {
            kind: "point",
            coordinates: { x: "1.2300", y: "4.5600", z: null, measure: null },
          },
          attributes: { parcel_id: { kind: "number", value: "124" } },
        },
      ],
      has_more: true,
    });
    const client = new SemanticClient({ request } as unknown as SemanticTransport);
    const page = await client.geoFeatures({
      project: "project-1",
      path: "parcels.gpkg",
      snapshot: "snapshot-2",
      layer: "layer-3",
      offset: 0,
      limit: 50,
    });
    expect(request).toHaveBeenCalledWith(
      "geo_features",
      expect.objectContaining({ path: "parcels.gpkg", snapshot: "snapshot-2", limit: 50 }),
      undefined,
    );
    expect(page).toMatchObject({
      project: "project-1",
      snapshot: "snapshot-2",
      layer: "layer-3",
      hasMore: true,
      features: [
        {
          id: "feature-4",
          nativeId: "124",
          geometry: { kind: "point", coordinates: { x: "1.2300", y: "4.5600" } },
          attributes: { parcel_id: { kind: "number", value: "124" } },
        },
      ],
    });
  });

  it("rejects untyped or malformed geometry instead of coercing coordinates", async () => {
    const client = new SemanticClient({
      request: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        layer: "layer-3",
        features: [
          {
            id: "feature-4",
            attributes: {},
            geometry: { kind: "point", coordinates: { x: 1.23, y: "4.56" } },
          },
        ],
        has_more: false,
      }),
    } as unknown as SemanticTransport);
    await expect(
      client.geoFeatures({
        project: "project-1",
        path: "parcels.gpkg",
        snapshot: "snapshot-2",
        layer: "layer-3",
        offset: 0,
        limit: 50,
      }),
    ).rejects.toThrow("x coordinate must be a string");
  });

  it("normalizes spatial query provenance and exact edit candidate handles", async () => {
    const request = vi
      .fn()
      .mockResolvedValueOnce({
        project: "project-1",
        snapshot: "snapshot-2",
        layer: "layer-3",
        features: [],
        has_more: false,
        source_revision: "revision-4",
        source_digest: "cd".repeat(32),
        provider: "geopackage",
        layer_crs: { authority: "EPSG", code: "4326" },
        strategy: "attributes_in_snapshot_feature_page",
        coordinate_transform: null,
        diagnostics: [],
      })
      .mockResolvedValueOnce({
        candidate: "candidate-5",
        project: "project-1",
        base_snapshot: "snapshot-2",
        layer: "layer-3",
        feature: "feature-6",
        path: "parcels.gpkg",
      })
      .mockResolvedValueOnce({
        candidate: "candidate-7",
        project: "project-1",
        base_snapshot: "snapshot-2",
        layer: "layer-3",
        feature: "feature-6",
        field: "area",
        path: "parcels.gpkg",
      })
      .mockResolvedValueOnce({
        project: "project-1",
        path: "project.qgs",
        snapshot: "snapshot-2",
        revision: "revision-8",
        id: "qgis-project-9",
        title: "Planning",
        crs: { authority: "EPSG", code: "4326" },
        layers: [
          {
            id: "qgis-reference-10",
            name: "Parcels",
            data_source: "./parcels.gpkg|layername=parcels",
            path: "parcels.gpkg",
            resolved_path: "parcels.gpkg",
            source_layer: "parcels",
            dataset: "dataset-1",
            layer: "layer-3",
            resolution_diagnostic: null,
            visible: true,
            renderer: "categorized",
            scale_range: null,
          },
        ],
        groups: [{ id: "group-11", name: "Planning", layers: ["qgis-reference-10"] }],
      });
    const client = new SemanticClient({ request } as unknown as SemanticTransport);
    const query = await client.queryGeoFeatures({
      project: "project-1",
      path: "parcels.gpkg",
      snapshot: "snapshot-2",
      layer: "layer-3",
      predicate: {
        kind: "envelope_overlaps",
        bounds: {
          minX: "1",
          minY: "2",
          maxX: "3",
          maxY: "4",
          crs: { authority: "EPSG", code: "4326" },
        },
      },
      offset: 0,
      limit: 100,
    });
    expect(query).toMatchObject({
      sourceRevision: "revision-4",
      provider: "geopackage",
      layerCrs: { authority: "EPSG", code: "4326" },
      diagnostics: [],
    });
    expect(request).toHaveBeenNthCalledWith(
      1,
      "geo_feature_query",
      expect.objectContaining({ layer: "layer-3", limit: 100 }),
      undefined,
    );
    const geometry = await client.proposeGeoGeometryEdit({
      project: "project-1",
      path: "parcels.gpkg",
      snapshot: "snapshot-2",
      layer: "layer-3",
      feature: "feature-6",
      geometry: { kind: "point", coordinates: { x: "2.5", y: "3.5" } },
    });
    const attribute = await client.proposeGeoAttributeEdit({
      project: "project-1",
      path: "parcels.gpkg",
      snapshot: "snapshot-2",
      layer: "layer-3",
      feature: "feature-6",
      field: "area",
      value: { kind: "number", value: "11.5" },
    });
    expect(geometry).toMatchObject({ candidate: "candidate-5", baseSnapshot: "snapshot-2" });
    expect(attribute).toMatchObject({ candidate: "candidate-7", field: "area" });
    const qgis = await client.inspectQgisProject({
      project: "project-1",
      path: "project.qgs",
      snapshot: "snapshot-2",
    });
    expect(qgis.layers[0]).toMatchObject({
      id: "qgis-reference-10",
      layer: "layer-3",
      resolvedPath: "parcels.gpkg",
    });
  });
});
