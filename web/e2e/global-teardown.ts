/** @file Stops the local services after browser tests complete. */

import { readFile, rm } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import type { FullConfig } from "@playwright/test";

const repositoryRoot = resolve(process.cwd(), "..");

/** Stops only services owned by the repository development launcher. */
/**
 * @param _config Playwright global configuration.
 * @returns Resolves after the stop request is issued.
 */
export default async function globalTeardown(_config: FullConfig): Promise<void> {
  spawnSync("python", ["tools/dev.py", "stop"], {
    cwd: repositoryRoot,
    stdio: "ignore",
    windowsHide: true,
  });
  try {
    const fixtureRoot = await readFile(
      resolve(repositoryRoot, "output/playwright/golden-fixture-root"),
      "utf8",
    );
    await rm(fixtureRoot.trim(), { recursive: true, force: true });
    await rm(resolve(repositoryRoot, "output/playwright/golden-fixture-root"), { force: true });
  } catch {
    // Preserve the original teardown behavior when the fixture was not created.
  }
}
