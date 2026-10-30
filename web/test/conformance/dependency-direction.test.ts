/** @file Verifies MUI dependency isolation. */
import { expect, it } from "vitest";
import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { packageText } from "./test-helpers";
it("allows MUI only in its package boundary", () => {
  expect(packageText("components-mui")).toContain("@mui/");
  expect(packageText("components")).not.toContain("@mui/");
});

it("keeps reusable packages independent from integrated applications", () => {
  const packages = resolve(import.meta.dirname, "../../packages");
  for (const entry of readdirSync(packages, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const manifest = JSON.parse(
      readFileSync(resolve(packages, entry.name, "package.json"), "utf8"),
    ) as { dependencies?: Record<string, string>; devDependencies?: Record<string, string> };
    const dependencies = {
      ...manifest.dependencies,
      ...manifest.devDependencies,
    };
    expect(dependencies, entry.name).not.toHaveProperty("@scriberig/app");
    expect(packageText(entry.name), entry.name).not.toMatch(
      /(?:\.\.\/)+apps\/scriberig|@scriberig\/web-app/,
    );
  }
});
