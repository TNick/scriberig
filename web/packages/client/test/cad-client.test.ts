/** @file Verifies the CAD candidate client method and wire normalization. */
import { describe, expect, it, vi } from "vitest";
import { SemanticClient } from "../src/semantic-client";
import type { SemanticTransport } from "../src/transport";

describe("CAD semantic client", () => {
  it("requests an exact-base line edit and normalizes its candidate", async () => {
    const request = vi.fn().mockResolvedValue({
      candidate: "candidate-4",
      project: "project-1",
      base_snapshot: "snapshot-2",
      path: "drawing.dxf",
      validation: { state: "pending" },
    });
    const client = new SemanticClient({ request } as unknown as SemanticTransport);
    const candidate = await client.proposeCadLineEdit({
      project: "project-1",
      snapshot: "snapshot-2",
      entity: "line-3",
      start: { x: "1", y: "2", z: "0" },
      end: { x: "3", y: "4", z: "0" },
    });
    expect(request).toHaveBeenCalledWith(
      "cad_line_edit",
      expect.objectContaining({
        snapshot: "snapshot-2",
        entity: "line-3",
      }),
      undefined,
    );
    expect(candidate).toMatchObject({
      candidateId: "candidate-4",
      baseSnapshot: "snapshot-2",
      validation: { status: "pending" },
    });
  });

  it("requests an exact-base layer change and normalizes its candidate", async () => {
    const request = vi.fn().mockResolvedValue({
      candidate: "candidate-5",
      project: "project-1",
      base_snapshot: "snapshot-2",
      path: "drawing.dxf",
      validation: { state: "passed" },
    });
    const client = new SemanticClient({ request } as unknown as SemanticTransport);
    const candidate = await client.proposeCadLayerChange({
      project: "project-1",
      snapshot: "snapshot-2",
      entity: "line-3",
      destinationLayer: "labels-4",
    });
    expect(request).toHaveBeenCalledWith(
      "cad_layer_change",
      expect.objectContaining({ destinationLayer: "labels-4", snapshot: "snapshot-2" }),
      undefined,
    );
    expect(candidate).toMatchObject({
      candidateId: "candidate-5",
      baseSnapshot: "snapshot-2",
      validation: { status: "passed" },
    });
  });
});
