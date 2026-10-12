/** @file Defines application routes independent of semantic identity. */
export type AppRoute =
  | "projects"
  | "raw"
  | "semantic"
  | "projection"
  | "history"
  | "session"
  | "migration"
  | "operations";

/** Parses a route with a safe fallback. */
/** @param value Route value. @returns Normalized application route. */
export function parseRoute(value: string): AppRoute {
  return [
    "projects",
    "raw",
    "semantic",
    "projection",
    "history",
    "session",
    "migration",
    "operations",
  ].includes(value)
    ? (value as AppRoute)
    : "raw";
}
