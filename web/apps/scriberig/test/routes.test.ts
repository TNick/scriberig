/** @file Verifies semantic symbols route to their rich artifact views. */

import { describe, expect, it } from "vitest";
import { semanticRouteForKind } from "../src/routes";
import { pathnameForRoute, routeFromPathname } from "../src/routes";
import { navigationGroups } from "../src/app-shell";

describe("workbench navigation", () => {
  it("groups every supported route exactly once", () => {
    const routes = navigationGroups.flatMap((group) => group.routes.map((route) => route.id));
    expect(routes).toEqual([
      "projects",
      "artifacts",
      "semantic",
      "raw",
      "document",
      "cad",
      "spreadsheet",
      "geo",
      "projection",
      "history",
      "session",
      "migration",
      "operations",
    ]);
    expect(new Set(routes).size).toBe(routes.length);
  });

  it("maps canonical browser paths in both directions", () => {
    expect(pathnameForRoute("spreadsheet")).toBe("/spreadsheet");
    expect(routeFromPathname("/spreadsheet")).toBe("spreadsheet");
    expect(routeFromPathname("/")).toBe("projects");
    expect(routeFromPathname("/not-a-route")).toBe("projects");
  });
});

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
