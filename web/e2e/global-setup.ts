/** @file Starts the local API and Vite services before browser tests. */

import { spawn } from "node:child_process";
import { resolve } from "node:path";
import type { FullConfig } from "@playwright/test";

const repositoryRoot = resolve(process.cwd(), "..");

/** Waits until the managed service responds successfully. */
/**
 * @param url Service URL to poll.
 * @returns Resolves after the service responds successfully.
 * @throws When the service does not become ready before the timeout.
 */
async function waitFor(url: string): Promise<void> {
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The service is still starting.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 1_000));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

/** Starts and verifies the repository-managed development services. */
/**
 * @param _config Playwright global configuration.
 * @returns Resolves after both services are ready.
 */
export default async function globalSetup(_config: FullConfig): Promise<void> {
  const child = spawn("python", ["tools/dev.py", "start"], {
    cwd: repositoryRoot,
    detached: true,
    env: { ...process.env, SCRIBERIG_NO_BROWSER: "1" },
    stdio: "ignore",
    windowsHide: true,
  });
  child.unref();
  await waitFor("http://127.0.0.1:7337/api/health");
  await waitFor("http://127.0.0.1:5173");
}
