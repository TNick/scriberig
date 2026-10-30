/** @file Verifies Rust wire request encoding and response normalization. */
import { describe, expect, it, vi } from "vitest";
import { HttpTransport, ScriberigClient } from "../src";
describe("protocol wire contracts", () => {
  it("encodes inspection requests using the pinned Rust selector contract", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(body).toEqual({
        entity: { id: "entity-1" },
        snapshot: { Exact: { id: "snapshot-1" } },
        detail: "Compact",
      });
      return new Response(JSON.stringify({ response: { Inspect: { summary: "root" } } }), {
        status: 200,
      });
    });
    vi.stubGlobal("fetch", fetch);

    await new HttpTransport("http://test/api").request("inspect", {
      entityId: "entity-1",
      snapshot: { kind: "exact", snapshotId: "snapshot-1" },
      detail: "compact",
    });

    expect(fetch).toHaveBeenCalledOnce();
    vi.unstubAllGlobals();
  });

  it("encodes style setting scopes with the Rust enum and snake-case contract", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        scope: { View: { view_id: "source-view-1" } },
        projection_kind: "javascript",
        expected_revision: 4,
        style: [113, 117, 111, 116, 101, 61, 115, 105, 110, 103, 108, 101],
      });
      return new Response(
        JSON.stringify({ response: { ProjectionStyleSetting: { style: [113], revision: 5 } } }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const setting = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).setProjectionStyle({
      scope: { kind: "view", viewId: "source-view-1" },
      projectionKind: "javascript",
      expectedRevision: 4,
      style: Array.from(new TextEncoder().encode("quote=single")),
    });

    expect(setting).toEqual({ style: [113], revision: 5 });
    vi.unstubAllGlobals();
  });

  it("normalizes style schema, source provenance, and exact fidelity", async () => {
    const fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      if (String(url).endsWith("/projection_style_schema")) {
        expect(JSON.parse(String(init?.body))).toEqual({ projection_kind: "javascript" });
        return new Response(
          JSON.stringify({
            response: {
              ProjectionStyleSchema: {
                reference: { adapter_id: "ecma-source-style", version: "1" },
                projection_kinds: ["javascript", "typescript"],
                properties: [
                  {
                    key: "indent",
                    label: "indent",
                    description: null,
                    kind: "Enum",
                    allowed_values: ["spaces", "tabs"],
                    minimum: null,
                    maximum: null,
                    default: [115, 112, 97, 99, 101, 115],
                    applicability: "Both",
                    affects_fidelity: false,
                    required_for_materialization: false,
                  },
                ],
              },
            },
          }),
          { status: 200 },
        );
      }
      expect(JSON.parse(String(init?.body))).toEqual({
        projection_kind: "javascript",
        project_id: "project-1",
        workspace_id: "workspace-1",
        purpose: "InteractiveView",
        view_id: "source-view-1",
        explicit_personal: null,
      });
      return new Response(
        JSON.stringify({
          response: {
            ProjectionStyleResolve: {
              purpose: "InteractiveView",
              reference: "Inline",
              value: [113, 117, 111, 116, 101, 61, 115, 105, 110, 103, 108, 101],
              schema: { adapter_id: "ecma-source-style", version: "1" },
              applied: [
                {
                  kind: "user-preference",
                  identity: "alice",
                  revision: null,
                  generation: 3,
                  value: [113, 117, 111, 116, 101, 61, 115, 105, 110, 103, 108, 101],
                },
              ],
              fidelity: {
                fidelity: "LosslessEditable",
                omitted_categories: [],
                explanation: null,
              },
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const client = new ScriberigClient(new HttpTransport("http://test/api"));
    const schema = await client.projectionStyleSchema("javascript");
    expect(schema?.properties[0]).toMatchObject({ key: "indent", kind: "enum" });
    const style = await client.resolveProjectionStyle({
      projectionKind: "javascript",
      projectId: "project-1",
      workspaceId: "workspace-1",
      purpose: "interactiveView",
      viewId: "source-view-1",
    });
    expect(style).toMatchObject({
      purpose: "interactiveView",
      reference: { kind: "inline" },
      applied: [{ kind: "user-preference", identity: "alice", generation: 3 }],
      fidelity: { fidelity: "losslessEditable", omittedCategories: [] },
    });
    vi.unstubAllGlobals();
  });

  it("normalizes the pinned Rust inspection response into semantic objects", async () => {
    const fetch = vi.fn(
      async () =>
        new Response(
          JSON.stringify({
            response: {
              Inspect: {
                entity: { id: "entity-1" },
                revision: { entity: { id: "entity-1" }, id: "revision-1" },
                snapshot: { id: "snapshot-1" },
                object_type: "rust.function",
                summary: "Add two values",
              },
            },
          }),
          { status: 200 },
        ),
    );
    vi.stubGlobal("fetch", fetch);

    const inspected = await new ScriberigClient(new HttpTransport("http://test/api")).inspect({
      entityId: "entity-1" as never,
      snapshot: { kind: "exact", snapshotId: "snapshot-1" as never },
      detail: "compact",
    });

    expect(inspected).toMatchObject({
      summary: {
        ref: { entityId: "entity-1", snapshotId: "snapshot-1", revisionId: "revision-1" },
        typeId: "rust.function",
        label: "Add two values",
      },
      text: "Add two values",
    });
    vi.unstubAllGlobals();
  });

  it("encodes child listings and normalizes universal navigation entries", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        specification: {
          subject: {
            entity: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] },
            snapshot: { bytes: Array.from({ length: 31 }, () => 0).concat(1) },
          },
          collection: {
            owner: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] },
            collection_type: "children",
          },
          page: { limit: 100, continuation: null },
          detail: "Compact",
        },
      });
      return new Response(
        JSON.stringify({
          response: {
            List: {
              result: {
                snapshot: { bytes: Array.from({ length: 31 }, () => 0).concat(1) },
                page: {
                  items: [
                    {
                      target: {
                        Snapshot: {
                          entity: { bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2] },
                          snapshot: { bytes: Array.from({ length: 31 }, () => 0).concat(1) },
                        },
                      },
                      entity_type: "rust.function",
                      label: "add",
                    },
                  ],
                  next: null,
                },
              },
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const page = await new ScriberigClient(new HttpTransport("http://test/api")).list({
      entityId: "ent_00000000000000000000000000000001" as never,
      snapshot: { kind: "exact", snapshotId: `snp_${"0".repeat(62)}01` as never },
      relation: "children",
    });

    expect(page).toMatchObject({
      items: [
        {
          ref: {
            entityId: "ent_00000000000000000000000000000002",
            snapshotId: `snp_${"0".repeat(62)}01`,
          },
          typeId: "rust.function",
          label: "add",
        },
      ],
    });
    vi.unstubAllGlobals();
  });

  it("encodes spatial query bounds with the Rust CRS envelope contract", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        path: "parcels.gpkg",
        snapshot: "snapshot-2",
        layer: "layer-3",
        predicate: {
          kind: "envelope_overlaps",
          bounds: {
            min_x: "1.00",
            min_y: "2.00",
            max_x: "3.00",
            max_y: "4.00",
            crs: { authority: "EPSG", code: "4326" },
          },
        },
        offset: 0,
        limit: 100,
      });
      return new Response(
        JSON.stringify({
          response: {
            GeoFeatureQuery: {
              project: "project-1",
              snapshot: "snapshot-2",
              layer: "layer-3",
              features: [],
              has_more: false,
              source_revision: "revision-4",
              source_digest: "ef".repeat(32),
              provider: "geopackage",
              layer_crs: { authority: "EPSG", code: "4326" },
              strategy: "spatial_index",
              coordinate_transform: null,
              diagnostics: [],
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const result = await new ScriberigClient(new HttpTransport("http://test/api")).queryGeoFeatures(
      {
        project: "project-1",
        path: "parcels.gpkg",
        snapshot: "snapshot-2",
        layer: "layer-3",
        predicate: {
          kind: "envelope_overlaps",
          bounds: {
            minX: "1.00",
            minY: "2.00",
            maxX: "3.00",
            maxY: "4.00",
            crs: { authority: "EPSG", code: "4326" },
          },
        },
        offset: 0,
        limit: 100,
      },
    );

    expect(result.sourceRevision).toBe("revision-4");
    vi.unstubAllGlobals();
  });

  it("encodes and normalizes snapshot-bound CAD bounds queries", async () => {
    const fetch = vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-3",
        space: "Model",
        bounds: { min_x: "0", min_y: "1", max_x: "2", max_y: "3" },
        offset: 0,
        limit: 25,
      });
      return new Response(
        JSON.stringify({
          response: {
            CadBoundsQuery: {
              project: "project-1",
              snapshot: "snapshot-2",
              drawing: "drawing-3",
              space: "Model",
              bounds: { min_x: "0", min_y: "1", max_x: "2", max_y: "3" },
              entities: [],
              total: 0,
              has_more: false,
              unbounded_entities: 0,
              complete: true,
            },
          },
        }),
        { status: 200 },
      );
    });
    vi.stubGlobal("fetch", fetch);

    const page = await new ScriberigClient(
      new HttpTransport("http://test/api"),
    ).cadEntitiesIntersectingBounds({
      project: "project-1",
      snapshot: "snapshot-2",
      drawing: "drawing-3",
      space: "Model",
      bounds: { minX: "0", minY: "1", maxX: "2", maxY: "3" },
      offset: 0,
      limit: 25,
    });

    expect(page).toMatchObject({
      project: "project-1",
      snapshot: "snapshot-2",
      drawing: "drawing-3",
      space: "Model",
      bounds: { minX: "0", minY: "1", maxX: "2", maxY: "3" },
      entities: [],
      total: 0,
      hasMore: false,
      unboundedEntities: 0,
      complete: true,
    });
    vi.unstubAllGlobals();
  });
});
