/** @file Defines application routes independent of semantic identity. */
export type AppRoute = "raw" | "semantic" | "projection" | "history" | "session" | "operations";

/** Parses a route with a safe fallback. */
/** @param value Route value. @returns Normalized application route. */
export function parseRoute(value: string): AppRoute {
  return ["raw", "semantic", "projection", "history", "session", "operations"].includes(value)
    ? (value as AppRoute)
    : "raw";
}
