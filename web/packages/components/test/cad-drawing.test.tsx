/** @file Verifies reusable CAD drawing and candidate controls. */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CadDrawingView } from "../src";

describe("CAD drawing component", () => {
  it("renders typed CAD layer and line semantics with a candidate action", () => {
    const html = renderToStaticMarkup(
      <CadDrawingView
        client={{} as never}
        controller={
          {
            currentDrawing: () => ({
              project: "project-1",
              snapshot: "snapshot-1",
              drawing: "drawing-1",
              units: "millimeter",
              layers: [
                { id: "layer-1", name: "PARCELS", visible: true, locked: false },
                { id: "layer-2", name: "LABELS", visible: true, locked: false },
              ],
              entityCount: 1,
              complete: true,
              diagnostics: [],
            }),
            currentEntities: () => ({
              project: "project-1",
              snapshot: "snapshot-1",
              drawing: "drawing-1",
              total: 1,
              hasMore: false,
              entities: [
                {
                  id: "line-1",
                  layer: "layer-1",
                  space: "Model",
                  geometry: {
                    Line: { start: { x: "0", y: "0", z: "0" }, end: { x: "1", y: "1", z: "0" } },
                  },
                },
                {
                  id: "circle-1",
                  layer: "layer-1",
                  space: "Model",
                  geometry: {
                    Circle: {
                      center: { x: "1.20", y: "0.50", z: "0" },
                      radius: "0.30",
                    },
                  },
                },
                {
                  id: "arc-1",
                  layer: "layer-1",
                  space: "Model",
                  geometry: {
                    Arc: {
                      center: { x: "-2.1", y: "1", z: "0" },
                      radius: "0.5",
                      start_angle: "0",
                      end_angle: "90",
                    },
                  },
                },
              ],
            }),
          } as never
        }
      />,
    );
    expect(html).toContain("PARCELS");
    expect(html).toContain("Propose layer change");
    expect(html).toContain("LABELS");
    expect(html).toContain("Line");
    expect(html).toContain("Create candidate");
    expect(html).toContain('<svg role="group" aria-label="Semantic CAD entities"');
    expect(html).toContain('aria-label="Select CAD entity line-1"');
    expect(html).toContain('data-min-x="0"');
    expect(html).toContain('data-max-y="1"');
    expect(html).toContain('data-min-x="0.9"');
    expect(html).toContain('data-max-x="1.5"');
    expect(html).toContain('aria-label="Select CAD arc bounds for entity arc-1"');
    expect(html).toContain('data-min-x="-2.6"');
    expect(html).toContain('aria-label="Zoom in"');
    expect(html).toContain('aria-label="Pan left"');
    expect(html).toContain("Reset view");
  });
});
