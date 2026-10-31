/** @file Defines application routes independent of semantic identity. */
export type AppRoute =
  | "projects"
  | "artifacts"
  | "document"
  | "cad"
  | "spreadsheet"
  | "geo"
  | "raw"
  | "semantic"
  | "projection"
  | "history"
  | "session"
  | "migration"
  | "operations";

const appRoutes: AppRoute[] = [
  "projects",
  "artifacts",
  "document",
  "cad",
  "spreadsheet",
  "geo",
  "raw",
  "semantic",
  "projection",
  "history",
  "session",
  "migration",
  "operations",
];

/**
 * Parses a route with a safe fallback.
 * @param value Route value.
 * @returns Normalized application route.
 * @throws This pure parser does not throw.
 */
export function parseRoute(value: string): AppRoute {
  return appRoutes.includes(value as AppRoute) ? (value as AppRoute) : "raw";
}

/** Resolves a browser pathname to an application route.
 * @param pathname Pathname from the current browser location.
 * @returns The matching route, defaulting the root and unknown paths to Projects.
 * @throws This path parser does not throw.
 */
export function routeFromPathname(pathname: string): AppRoute {
  const segment = pathname.replace(/^\/+|\/+$/g, "");
  return segment === ""
    ? "projects"
    : appRoutes.includes(segment as AppRoute)
      ? (segment as AppRoute)
      : "projects";
}

/** Returns the canonical browser path for an application route.
 * @param route Route to serialize.
 * @returns Canonical root-relative pathname.
 * @throws This pure path formatter does not throw.
 */
export function pathnameForRoute(route: AppRoute): string {
  return `/${route}`;
}

/**
 * Resolves a semantic symbol kind to its integrated rich artifact view.
 * @param kind Exact semantic object type reported by the active project.
 * @returns The matching domain route, or `undefined` when no rich view owns the type.
 * @throws This lookup does not throw.
 */
export function semanticRouteForKind(kind: string): AppRoute | undefined {
  switch (kind) {
    case "document.root":
      return "document";
    case "cad.drawing":
      return "cad";
    case "spreadsheet.worksheet":
      return "spreadsheet";
    case "geo.dataset":
    case "geo.layer":
    case "geo.feature":
    case "qgis.project":
    case "qgis.layer_reference":
      return "geo";
    default:
      return undefined;
  }
}
