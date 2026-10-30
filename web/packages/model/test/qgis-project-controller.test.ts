/** @file Verifies exact-snapshot QGIS project and rename orchestration. */
import { describe, expect, it, vi } from "vitest";
import type { QgisProjectResponse } from "@scriberig/protocol";
import { QgisProjectController } from "../src/qgis-project-controller";

const projectView: QgisProjectResponse = {
  project: "project-1",
  path: "project.qgs",
  snapshot: "snapshot-2",
  revision: "revision-3",
  id: "qgis-project-4",
  layers: [
    {
      id: "reference-5",
      name: "Parcels",
      dataSource: "parcels.gpkg",
      visible: true,
    },
    {
      id: "reference-6",
      name: "Roads",
      dataSource: "roads.gpkg",
      visible: true,
    },
  ],
  groups: [{ id: "group-7", name: "root", layers: ["reference-5", "reference-6"] }],
};

/** Creates typed client methods used by the controller.
 * @returns Mocked exact project and candidate operations.
 */
function client() {
  return {
    inspectQgisProject: vi.fn(async () => projectView),
    proposeQgisLayerRename: vi.fn(async () => ({
      candidate: "candidate-6",
      project: "project-1",
      baseSnapshot: "snapshot-2",
      reference: "reference-5",
      path: "project.qgs",
    })),
    proposeQgisLayerOrder: vi.fn(async () => ({
      candidate: "candidate-8",
      project: "project-1",
      baseSnapshot: "snapshot-2",
      path: "project.qgs",
    })),
  };
}

describe("QgisProjectController", () => {
  it("proposes only a complete exact-snapshot flat-tree permutation", async () => {
    const api = client();
    const controller = new QgisProjectController(api, "project-1", "project.qgs", "snapshot-2");
    await expect(controller.reorderLayers(["reference-5"])).rejects.toThrow(
      "Read the exact QGIS project",
    );
    await controller.read();
    await expect(controller.reorderLayers([])).rejects.toThrow("complete unique permutation");
    await expect(controller.reorderLayers(["reference-5", "reference-6"])).rejects.toThrow(
      "change the current order",
    );
    const candidate = await controller.reorderLayers(["reference-6", "reference-5"]);
    expect(candidate.candidate).toBe("candidate-8");
    expect(api.proposeQgisLayerOrder).toHaveBeenCalledWith(
      {
        project: "project-1",
        snapshot: "snapshot-2",
        path: "project.qgs",
        orderedReferences: ["reference-6", "reference-5"],
      },
      undefined,
    );
  });

  it("guards exact project-tree membership before proposing a rename", async () => {
    const api = client();
    const controller = new QgisProjectController(api, "project-1", "project.qgs", "snapshot-2");
    await expect(controller.renameReference("reference-5", "Lots")).rejects.toThrow(
      "Read the exact QGIS project",
    );
    await controller.read();
    const candidate = await controller.renameReference("reference-5", "Lots");
    expect(api.proposeQgisLayerRename).toHaveBeenCalledWith(
      {
        project: "project-1",
        snapshot: "snapshot-2",
        reference: "reference-5",
        newName: "Lots",
      },
      undefined,
    );
    expect(candidate.candidate).toBe("candidate-6");
    await expect(controller.renameReference("foreign-reference", "Lots")).rejects.toThrow(
      RangeError,
    );
  });

  it("rejects an inspection response outside the requested exact scope", async () => {
    const api = client();
    api.inspectQgisProject.mockResolvedValue({ ...projectView, snapshot: "other-snapshot" });
    const controller = new QgisProjectController(api, "project-1", "project.qgs", "snapshot-2");
    await expect(controller.read()).rejects.toThrow("does not match the requested snapshot");
    expect(controller.currentProject()).toBeUndefined();
  });

  it("rejects a candidate returned for another layer reference", async () => {
    const api = client();
    api.proposeQgisLayerRename.mockResolvedValue({
      candidate: "candidate-6",
      project: "project-1",
      baseSnapshot: "snapshot-2",
      reference: "other-reference",
      path: "project.qgs",
    });
    const controller = new QgisProjectController(api, "project-1", "project.qgs", "snapshot-2");
    await controller.read();
    await expect(controller.renameReference("reference-5", "Lots")).rejects.toThrow(
      "does not match the requested project scope",
    );
  });
});
