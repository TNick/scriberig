/** @file Verifies reusable packages expose declaration-producing builds. */
import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";

const packages = [
  "protocol",
  "client",
  "model",
  "editor-core",
  "react",
  "components",
  "components-mui",
  "agent-ui",
] as const;

it("defines independently buildable declaration-emitting packages", () => {
  for (const packageName of packages) {
    const packageDirectory = resolve(import.meta.dirname, "../../packages", packageName);
    const manifest = JSON.parse(
      readFileSync(resolve(packageDirectory, "package.json"), "utf8"),
    ) as {
      files?: string[];
      scripts?: { build?: string };
      publishConfig?: { exports?: Record<string, unknown> };
    };
    const buildConfig = resolve(packageDirectory, "tsconfig.build.json");
    const runtimeEntry = resolve(packageDirectory, "dist/index.js");
    const declarationEntry = resolve(packageDirectory, "dist/index.d.ts");

    expect(existsSync(buildConfig), `${packageName} build config`).toBe(true);
    expect(existsSync(runtimeEntry), `${packageName} runtime entry`).toBe(true);
    expect(existsSync(declarationEntry), `${packageName} declaration entry`).toBe(true);
    expect(manifest.scripts?.build, `${packageName} build script`).toContain("tsconfig.build.json");
    expect(manifest.scripts?.build, `${packageName} build emits`).not.toContain("--noEmit");
    expect(manifest.files, `${packageName} published files`).toContain("dist");
    expect(manifest.publishConfig?.exports?.["."], `${packageName} exports`).toEqual({
      types: "./dist/index.d.ts",
      import: "./dist/index.js",
    });
  }
});
