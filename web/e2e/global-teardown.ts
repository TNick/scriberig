/** @file Stops the local services after browser tests complete. */

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import type { FullConfig } from "@playwright/test";

const repositoryRoot = resolve(process.cwd(), "..");

/** Stops only services owned by the repository development launcher. */
export default async function globalTeardown(_config: FullConfig): Promise<void> {
  spawnSync("python", ["tools/dev.py", "stop"], {
    cwd: repositoryRoot,
    stdio: "ignore",
    windowsHide: true,
  });
}
