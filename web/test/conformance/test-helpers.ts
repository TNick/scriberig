/** @file Shared helpers for web architecture conformance tests. */
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
/** Reads a package source tree as one string.\n * @param packageName Workspace package name.\n * @returns Concatenated source text.\n */
export function packageText(packageName: string): string {
  const dir = resolve(import.meta.dirname, "../../packages", packageName, "src");
  const walk = (d: string): string =>
    readdirSync(d, { withFileTypes: true })
      .map((e) =>
        e.isDirectory()
          ? walk(resolve(d, e.name))
          : e.name.endsWith(".ts") || e.name.endsWith(".tsx")
            ? readFileSync(resolve(d, e.name), "utf8")
            : "",
      )
      .join("\n");
  return walk(dir);
}
