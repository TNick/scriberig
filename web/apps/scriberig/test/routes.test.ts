/** @file Verifies semantic symbols route to their rich artifact views. */

import { describe, expect, it } from "vitest";
import { semanticRouteForKind } from "../src/routes";

describe("semanticRouteForKind", () => {
  it("opens each imported rich domain from its semantic symbol", () => {
    expect(semanticRouteForKind("document.root")).toBe("document");
    expect(semanticRouteForKind("cad.drawing")).toBe("cad");
    expect(semanticRouteForKind("spreadsheet.worksheet")).toBe("spreadsheet");
    expect(semanticRouteForKind("geo.dataset")).toBe("geo");
    expect(semanticRouteForKind("geo.layer")).toBe("geo");
    expect(semanticRouteForKind("geo.feature")).toBe("geo");
    expect(semanticRouteForKind("qgis.project")).toBe("geo");
    expect(semanticRouteForKind("qgis.layer_reference")).toBe("geo");
  });

  it("leaves unrelated semantic symbols on the semantic view", () => {
    expect(semanticRouteForKind("rust.function")).toBeUndefined();
    expect(semanticRouteForKind("external_artifact")).toBeUndefined();
  });
});
