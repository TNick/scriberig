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
});
