/** @file Tests semantic selection and command discovery. */
import { describe, expect, it } from "vitest";
import normalizationFixture from "../../../../tests/fixtures/agent-selection-normalization.json";
import type { EntityId, ProjectionAnchor, RevisionId, SnapshotId } from "@scriberig/protocol";
import {
  SelectionController,
  sameSelection,
  selectDocumentPart,
  selectLinkedQgisGeoLayer,
  selectQgisLayerReference,
  selectCadEntity,
  selectGeoFeature,
  selectSpreadsheetCell,
  selectSpreadsheetRange,
} from "../src/selection";
import { discoverCommands } from "../src/commands";
import { agentSelectionFromSemanticSelection } from "../src/agent-selection";

describe("editor core primitives", () => {
  it("keeps the owning document when selecting a paragraph or table cell", () => {
    const document = { entityId: "ent_doc" as EntityId, snapshotId: "snp_a" as SnapshotId };
    const paragraph = selectDocumentPart(document, "ent_paragraph", "snp_a");
    const cell = selectDocumentPart(paragraph, "ent_cell", "snp_a");
    expect(paragraph).toEqual({
      entityId: "ent_paragraph",
      snapshotId: "snp_a",
      relationPath: ["ent_doc"],
    });
    expect(cell).toEqual({
      entityId: "ent_cell",
      snapshotId: "snp_a",
      relationPath: ["ent_doc"],
    });
  });

  it("preserves paragraph selection across view recreation and rerender", () => {
    const document = { entityId: "ent_doc" as EntityId, snapshotId: "snp_a" as SnapshotId };
    const firstViewParagraph = { id: "ent_paragraph" };
    const selected = selectDocumentPart(document, firstViewParagraph.id, "snp_a");
    const recreatedViewParagraph = { id: "ent_paragraph" };
    const restored = selectDocumentPart(document, recreatedViewParagraph.id, "snp_a");

    expect(restored).toEqual(selected);
    expect(sameSelection(restored, selected)).toBe(true);
  });

  it("keeps QGIS layer-reference and linked GeoLayer identities separate", () => {
    const project = { entityId: "ent_qgis_project" as EntityId, snapshotId: "snp_a" as SnapshotId };
    const reference = selectQgisLayerReference(
      project,
      "ent_qgis_layer_ref" as EntityId,
      project.snapshotId,
      "ent_geo_layer" as EntityId,
    );

    expect(reference.entityId).toBe("ent_qgis_layer_ref");
    expect(reference.qgisLayer?.linkedGeoLayerId).toBe("ent_geo_layer");

    const linked = selectLinkedQgisGeoLayer(reference);
    expect(linked?.entityId).toBe("ent_geo_layer");
    expect(linked?.qgisLayer?.referenceId).toBe("ent_qgis_layer_ref");
    expect(linked?.relationPath).toEqual(["ent_qgis_project", "ent_qgis_layer_ref"]);
    expect(linked?.snapshotId).toBe(project.snapshotId);
  });

  it("selects a geospatial feature while preserving its exact layer context", () => {
    const selection = selectGeoFeature(
      "ent_feature" as EntityId,
      "snp_a" as SnapshotId,
      "ent_geo_layer" as EntityId,
    );
    expect(selection).toEqual({
      entityId: "ent_feature",
      snapshotId: "snp_a",
      relationPath: ["ent_geo_layer"],
    });
  });

  it("creates typed spreadsheet cell and range anchors without replacing semantic identity", () => {
    const cell = selectSpreadsheetCell(
      "ent_cell_b3" as EntityId,
      "snp_a" as SnapshotId,
      "ent_sheet" as EntityId,
      3,
      2,
    );
    expect(cell).toEqual({
      entityId: "ent_cell_b3",
      snapshotId: "snp_a",
      relationPath: ["ent_sheet"],
      anchor: {
        kind: "spreadsheetCell",
        worksheetId: "ent_sheet",
        row: 3,
        column: 2,
      },
    });

    const range = selectSpreadsheetRange(
      "ent_sheet" as EntityId,
      "snp_a" as SnapshotId,
      2,
      1,
      5,
      4,
    );
    expect(range.entityId).toBe("ent_sheet");
    expect(range.anchor).toEqual({
      kind: "spreadsheetRange",
      worksheetId: "ent_sheet",
      firstRow: 2,
      firstColumn: 1,
      lastRow: 5,
      lastColumn: 4,
    });
    expect(sameSelection(range, { entityId: range.entityId, snapshotId: range.snapshotId })).toBe(
      true,
    );
    expect(() =>
      selectSpreadsheetRange("ent_sheet" as EntityId, "snp_a" as SnapshotId, 5, 2, 4, 3),
    ).toThrow("spreadsheet range anchor coordinates are invalid");
  });

  it("does not navigate an unresolved QGIS layer reference to an absent GeoLayer", () => {
    const reference = selectQgisLayerReference(
      { entityId: "ent_qgis_project" as EntityId, snapshotId: "snp_a" as SnapshotId },
      "ent_unresolved_ref" as EntityId,
      "snp_a" as SnapshotId,
    );
    expect(selectLinkedQgisGeoLayer(reference)).toBeUndefined();
  });

  it("retains semantic identity while projection location changes", () => {
    const controller = new SelectionController();
    const selection = {
      entityId: "ent_a" as EntityId,
      snapshotId: "snp_a" as SnapshotId,
      revisionId: "rev_a" as RevisionId,
    };
    controller.set(selection);
    controller.updateProjection({ key: "file", location: "line-1" });
    expect(sameSelection(controller.get(), selection)).toBe(true);
  });
  it("retains semantic identity across spreadsheet and CRS anchor changes", () => {
    const controller = new SelectionController();
    const selection = {
      entityId: "ent_feature" as EntityId,
      snapshotId: "snp_a" as SnapshotId,
    };
    const cell: ProjectionAnchor = {
      kind: "spreadsheetCell",
      worksheetId: "ent_sheet" as EntityId,
      row: 3,
      column: 2,
    };
    controller.set(selection);
    controller.updateAnchor(cell);
    expect(sameSelection(controller.get(), selection)).toBe(true);
    controller.updateAnchor({
      kind: "geometryBounds",
      crs: "EPSG:4326",
      minX: "-1",
      minY: "-1",
      maxX: "1",
      maxY: "1",
    });
    expect(sameSelection(controller.get(), selection)).toBe(true);
    controller.updateAnchor({
      kind: "cadBounds",
      drawingId: "ent_drawing" as EntityId,
      space: "model",
      minX: "-1",
      minY: "-1",
      maxX: "1",
      maxY: "1",
    });
    expect(sameSelection(controller.get(), selection)).toBe(true);
  });

  it("refreshes a surviving entity to the new snapshot without stale view metadata", () => {
    const controller = new SelectionController();
    controller.set({
      entityId: "ent_feature" as EntityId,
      snapshotId: "snp_old" as SnapshotId,
      revisionId: "rev_old" as RevisionId,
      projection: { key: "file", location: "row-4" },
      anchor: {
        kind: "geometryBounds",
        crs: "EPSG:4326",
        minX: "0",
        minY: "0",
        maxX: "1",
        maxY: "1",
      },
    });

    const refreshed = controller.refreshSnapshot("snp_new" as SnapshotId, true);

    expect(refreshed).toEqual({
      entityId: "ent_feature",
      snapshotId: "snp_new",
    });
    expect(controller.get()).toEqual(refreshed);
  });

  it("clears selection when the selected entity is absent from the new snapshot", () => {
    const controller = new SelectionController();
    controller.set({ entityId: "ent_removed" as EntityId, snapshotId: "snp_old" as SnapshotId });

    expect(controller.refreshSnapshot("snp_new" as SnapshotId, false)).toBeUndefined();
    expect(controller.get()).toBeUndefined();
  });

  it("maps exact semantic scope to agent context without view-only anchors", () => {
    const semantic = selectSpreadsheetCell(
      "ent_cell" as EntityId,
      "snp_exact" as SnapshotId,
      "ent_sheet" as EntityId,
      3,
      2,
    );
    const selected = {
      ...semantic,
      projection: { key: "spreadsheet-grid", location: "viewport-row-3" },
    };

    expect(
      agentSelectionFromSemanticSelection(selected, "project-1", "ent_project_root" as EntityId),
    ).toEqual({
      projectId: "project-1",
      rootId: "ent_project_root",
      entityId: "ent_cell",
      snapshotId: "snp_exact",
    });
    expect(
      agentSelectionFromSemanticSelection(undefined, "project-1", "ent_root" as EntityId),
    ).toBe(undefined);
  });

  it("normalizes spreadsheet and CRS-aware feature scopes independently of view anchors", () => {
    for (const item of normalizationFixture.cases) {
      const semantic = item.semantic;
      const selection =
        item.name === "spreadsheet cell"
          ? selectSpreadsheetCell(
              semantic.entityId as EntityId,
              semantic.snapshotId as SnapshotId,
              item.anchor.worksheetId as EntityId,
              item.anchor.row ?? 1,
              item.anchor.column ?? 1,
            )
          : {
              ...selectGeoFeature(
                semantic.entityId as EntityId,
                semantic.snapshotId as SnapshotId,
                "ent_geo_layer" as EntityId,
              ),
              anchor: item.anchor as ProjectionAnchor,
            };
      const normalized = agentSelectionFromSemanticSelection(
        selection,
        semantic.projectId,
        semantic.rootId as EntityId,
      );
      const changedView = agentSelectionFromSemanticSelection(
        {
          ...selection,
          anchor: { kind: "opaque", adapter: "view", anchorKind: "pan", value: "changed" },
        },
        semantic.projectId,
        semantic.rootId as EntityId,
      );

      expect(normalized).toEqual({
        projectId: semantic.projectId,
        rootId: semantic.rootId,
        entityId: semantic.entityId,
        snapshotId: semantic.snapshotId,
      });
      expect(changedView).toEqual(normalized);
    }
  });

  it("keeps UI page-region coordinates out of semantic selection identity", () => {
    const controller = new SelectionController();
    const selection = {
      entityId: "ent_page" as EntityId,
      snapshotId: "snp_a" as SnapshotId,
    };
    controller.set(selection);
    controller.updateAnchor({
      kind: "pageRegion",
      page: 1,
      x: 0,
      y: 120,
      width: 640,
      height: 480,
    });
    expect(sameSelection(controller.get(), selection)).toBe(true);
    expect(controller.get()?.entityId).toBe(selection.entityId);
    expect(controller.get()?.snapshotId).toBe(selection.snapshotId);
  });

  it("selects a CAD entity by semantic ID and keeps exact bounds in its anchor", () => {
    const selected = selectCadEntity(
      "ent_line" as EntityId,
      "snp_a" as SnapshotId,
      "ent_drawing" as EntityId,
      "model",
      { minX: "-1.25", minY: "0", maxX: "4.75", maxY: "8" },
    );
    expect(selected.entityId).toBe("ent_line");
    expect(selected.snapshotId).toBe("snp_a");
    expect(selected.relationPath).toEqual(["ent_drawing"]);
    expect(selected.anchor).toEqual({
      kind: "cadBounds",
      drawingId: "ent_drawing",
      space: "model",
      minX: "-1.25",
      minY: "0",
      maxX: "4.75",
      maxY: "8",
    });
  });
  it("filters commands by capability and applicability", () => {
    const commands = [
      {
        id: "read",
        title: "Read",
        mutating: false,
        requiredCapability: "inspect",
        execute: "inspect",
      },
    ];
    expect(discoverCommands(commands, undefined, new Set(["inspect"]))).toHaveLength(1);
    expect(discoverCommands(commands, undefined, new Set())).toHaveLength(0);
  });
});
