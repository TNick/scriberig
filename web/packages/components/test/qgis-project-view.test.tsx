/** @file Verifies QGIS project ordering controls use semantic references. */
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import type { QgisProjectResponse } from "@scriberig/protocol";
import { QgisProjectView } from "../src";

const project: QgisProjectResponse = {
  project: "project-1",
  path: "project.qgs",
  snapshot: "snapshot-2",
  revision: "revision-3",
  id: "qgis-project-4",
  layers: [
    {
      id: "reference-1",
      name: "Parcels",
      dataSource: "parcels.gpkg",
      visible: false,
      renderer: "categorized",
      scaleRange: "1:5000",
    },
    { id: "reference-2", name: "Roads", dataSource: "roads.gpkg", visible: true },
  ],
  groups: [{ id: "group-1", name: "root", layers: ["reference-1", "reference-2"] }],
};

describe("QgisProjectView", () => {
  it("shows semantic move controls only for a representable flat tree", () => {
    const html = renderToStaticMarkup(
      <QgisProjectView
        project={project}
        controller={{} as never}
        canBrowse={() => false}
        onBrowse={vi.fn()}
        onCandidate={vi.fn()}
        onError={vi.fn()}
      />,
    );
    expect(html).toContain("QGIS layer references");
    expect(html).toContain("Move up");
    expect(html).toContain("Move down");
    expect(html).toContain("Parcels");
    expect(html).toContain("Roads");
    expect(html).toContain("Project presentation for Parcels");
    expect(html).toContain("renderer kind categorized");
    expect(html).toContain("scale range 1:5000");
    expect(html).toContain("renderer metadata is retained as project configuration");
    expect(html).not.toContain("flat QGIS project trees");
  });

  it("explains when a nested tree cannot use the flat reorder operation", () => {
    const html = renderToStaticMarkup(
      <QgisProjectView
        project={{
          ...project,
          groups: [...project.groups, { id: "group-2", name: "nested", layers: [] }],
        }}
        controller={{} as never}
        canBrowse={() => false}
        onBrowse={vi.fn()}
        onCandidate={vi.fn()}
        onError={vi.fn()}
      />,
    );
    expect(html).toContain("flat QGIS project trees");
    expect(html).toContain('disabled=""');
  });
});
