/** @file Verifies snapshot-bound CAD reads and safe candidate targeting. */
import { describe, expect, it, vi } from "vitest";
import { CadPageController } from "../src/cad-page-controller";
import type { ScriberigClient } from "@scriberig/client";
import sharedCadTarget from "../../../../tests/fixtures/rich-artifact/cad-entity-target.json";

describe("CadPageController", () => {
  it("reads bounded semantics and proposes only loaded line entities", async () => {
    const client = {
      inspectCadDrawing: vi.fn().mockResolvedValue({
        project: sharedCadTarget.project,
        snapshot: sharedCadTarget.snapshot,
        drawing: sharedCadTarget.drawing,
        layers: [
          { id: sharedCadTarget.sourceLayer, name: "PARCELS", visible: true, locked: false },
          { id: sharedCadTarget.destinationLayer, name: "LABELS", visible: true, locked: false },
        ],
        complete: true,
      }),
      cadEntities: vi.fn().mockResolvedValue({
        project: sharedCadTarget.project,
        snapshot: sharedCadTarget.snapshot,
        drawing: sharedCadTarget.drawing,
        entities: [
          {
            id: sharedCadTarget.entity,
            layer: sharedCadTarget.sourceLayer,
            space: "model",
            geometry: { Line: {} },
          },
        ],
      }),
      proposeCadLineEdit: vi.fn().mockResolvedValue({
        candidateId: "candidate-1",
        project: sharedCadTarget.project,
        baseSnapshot: sharedCadTarget.snapshot,
      }),
      proposeCadLayerChange: vi.fn().mockResolvedValue({
        candidateId: "candidate-2",
        project: sharedCadTarget.project,
        baseSnapshot: sharedCadTarget.snapshot,
      }),
    } as unknown as ScriberigClient;
    const controller = new CadPageController(
      client,
      sharedCadTarget.project,
      sharedCadTarget.snapshot,
      sharedCadTarget.drawing,
    );
    await controller.readDrawing();
    await controller.readEntities(0, 10);
    await controller.replaceLine(
      sharedCadTarget.entity,
      { x: "0", y: "0", z: "0" },
      { x: "1", y: "1", z: "0" },
    );
    expect(client.proposeCadLineEdit).toHaveBeenCalledWith(
      expect.objectContaining({
        project: sharedCadTarget.project,
        snapshot: sharedCadTarget.snapshot,
        entity: sharedCadTarget.entity,
      }),
      undefined,
    );
    await controller.changeLayer(sharedCadTarget.entity, sharedCadTarget.destinationLayer);
    expect(client.proposeCadLayerChange).toHaveBeenCalledWith(
      {
        project: sharedCadTarget.project,
        snapshot: sharedCadTarget.snapshot,
        entity: sharedCadTarget.entity,
        destinationLayer: sharedCadTarget.destinationLayer,
      },
      undefined,
    );
    await expect(
      controller.replaceLine(
        "foreign-line",
        { x: "0", y: "0", z: "0" },
        { x: "1", y: "1", z: "0" },
      ),
    ).rejects.toThrow("not present");
  });

  it("rejects non-line geometry before requesting a candidate", async () => {
    const client = {
      inspectCadDrawing: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        layers: [],
        complete: true,
      }),
      cadEntities: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        entities: [{ id: "circle-1", geometry: { Circle: {} } }],
      }),
      proposeCadLineEdit: vi.fn(),
    } as unknown as ScriberigClient;
    const controller = new CadPageController(client, "project-1", "snapshot-2", "drawing-1");
    await controller.readDrawing();
    await controller.readEntities(0, 10);
    await expect(
      controller.replaceLine("circle-1", { x: "0", y: "0", z: "0" }, { x: "1", y: "1", z: "0" }),
    ).rejects.toThrow("LINE geometry only");
    expect(client.proposeCadLineEdit).not.toHaveBeenCalled();
  });

  it("refuses candidate operations when drawing fidelity is incomplete", async () => {
    const client = {
      inspectCadDrawing: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        layers: [],
        complete: false,
      }),
      cadEntities: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        entities: [{ id: "line-1", geometry: { Line: {} } }],
      }),
      proposeCadLineEdit: vi.fn(),
    } as unknown as ScriberigClient;
    const controller = new CadPageController(client, "project-1", "snapshot-2", "drawing-1");
    await controller.readDrawing();
    await controller.readEntities(0, 10);
    await expect(
      controller.replaceLine("line-1", { x: "0", y: "0", z: "0" }, { x: "1", y: "1", z: "0" }),
    ).rejects.toThrow("fidelity");
    expect(client.proposeCadLineEdit).not.toHaveBeenCalled();
  });

  it("rejects layer candidates for missing, locked, or mismatched layers", async () => {
    const client = {
      inspectCadDrawing: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        layers: [
          { id: "layer-1", name: "PARCELS", visible: true, locked: false },
          { id: "layer-locked", name: "LOCKED", visible: true, locked: true },
        ],
        complete: true,
      }),
      cadEntities: vi.fn().mockResolvedValue({
        project: "project-1",
        snapshot: "snapshot-2",
        drawing: "drawing-1",
        entities: [{ id: "line-1", layer: "layer-1", geometry: { Line: {} } }],
      }),
      proposeCadLayerChange: vi.fn(),
    } as unknown as ScriberigClient;
    const controller = new CadPageController(client, "project-1", "snapshot-2", "drawing-1");
    await controller.readDrawing();
    await controller.readEntities(0, 10);
    await expect(controller.changeLayer("line-1", "missing")).rejects.toThrow("not present");
    await expect(controller.changeLayer("line-1", "layer-locked")).rejects.toThrow("locked");
    expect(client.proposeCadLayerChange).not.toHaveBeenCalled();

    const mismatched = new CadPageController(
      {
        ...client,
        inspectCadDrawing: vi.fn().mockResolvedValue({
          project: "project-1",
          snapshot: "snapshot-other",
          drawing: "drawing-1",
          layers: [],
          complete: true,
        }),
      } as unknown as ScriberigClient,
      "project-1",
      "snapshot-2",
      "drawing-1",
    );
    await expect(mismatched.readDrawing()).rejects.toThrow("identity does not match");
    expect(mismatched.currentDrawing()).toBeUndefined();
  });
});
