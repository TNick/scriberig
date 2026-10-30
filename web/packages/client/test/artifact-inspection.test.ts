/** @file Verifies artifact fidelity survives client response normalization. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
import {
  normalizeArtifactInspectResponse,
  normalizeArtifactMembersResponse,
} from "../src/artifact-views";

describe("artifact client views", () => {
  it("marks missing synchronization fields from older servers as unavailable", () => {
    const response = normalizeArtifactInspectResponse({
      project: "project-1",
      artifacts: [
        {
          id: "artifact-1",
          logical_key: "roads",
          kind: "shapefile",
          adapter: "shapefile",
          scan_id: "scan-1",
          fidelity: "read-only-derived",
          opaque_preservation: "unproven",
          references: [],
          members: [],
          member_count: 0,
        },
      ],
      total: 1,
    });
    expect(response.artifacts[0]?.synchronizationStatus).toBe("unavailable");
    expect(response.artifacts[0]?.baselineDigest).toBe("");
  });

  it("reads bounded artifact summaries through the shared protocol", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          artifacts: [
            {
              id: "artifact-1",
              logical_key: "roads",
              kind: "shapefile",
              adapter: "dxf",
              adapter_version: null,
              scan_id: "scan-1",
              baseline_digest: "baseline-1",
              observed_digest: "baseline-1",
              synchronization_status: "in-sync",
              fidelity: "editable-with-opaque-preservation",
              opaque_preservation: "preserved",
              diagnostic: null,
              references: [],
              member_count: 22,
              members: [
                {
                  key: "geometry",
                  project_path: "roads.shp",
                  container_part: null,
                  digest: "digest-1",
                },
              ],
            },
          ],
          total: 1,
          next_offset: null,
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const response = await new ScriberigClient(transport).inspectArtifacts({
      project: "project-1",
      offset: 0,
      limit: 25,
    });
    expect(response.artifacts[0]?.logicalKey).toBe("roads");
    expect(response.artifacts[0]?.fidelity).toBe("editable-with-opaque-preservation");
    expect(response.artifacts[0]?.opaquePreservation).toBe("preserved");
    expect(response.artifacts[0]?.baselineDigest).toBe("baseline-1");
    expect(response.artifacts[0]?.synchronizationStatus).toBe("in-sync");
    expect(response.artifacts[0]?.members[0]?.projectPath).toBe("roads.shp");
    expect(calls).toEqual([["artifact_inspect", { project: "project-1", offset: 0, limit: 25 }]]);
  });

  it("reads and normalizes bounded container-member pages", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          artifact: "artifact-1",
          members: [
            {
              key: "container:xl/workbook.xml",
              project_path: null,
              container_part: "xl/workbook.xml",
              digest: "ab".repeat(32),
            },
          ],
          total: 41,
          next_offset: 20,
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const response = await new ScriberigClient(transport).inspectArtifactMembers({
      project: "project-1",
      artifact: "artifact-1",
      offset: 0,
      limit: 20,
    });
    expect(response.members[0]?.containerPart).toBe("xl/workbook.xml");
    expect(response.total).toBe(41);
    expect(response.nextOffset).toBe(20);
    expect(calls).toEqual([
      ["artifact_members", { project: "project-1", artifact: "artifact-1", offset: 0, limit: 20 }],
    ]);
    expect(
      normalizeArtifactMembersResponse({
        project: "project-1",
        artifact: "artifact-1",
        members: [],
        total: 0,
        next_offset: null,
      }).nextOffset,
    ).toBeUndefined();
  });
});
