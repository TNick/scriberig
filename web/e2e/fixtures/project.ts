/** @file Provides isolated project onboarding for browser acceptance tests. */

import { cp, mkdtemp, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { Page } from "@playwright/test";
import { expect } from "./app";

const repositoryRoot = resolve(process.cwd(), "..");
const fixtureRootMarker = resolve(repositoryRoot, "output/playwright/golden-fixture-root");

/** Creates and imports a copied Rust project into the managed browser service.
 * @param page Browser page already opened on the Projects view.
 * @param name Display name for the imported project.
 * @returns The isolated project directory used by the scan.
 * @throws When project setup, scan, or import fails.
 */
export async function importGoldenProject(page: Page, name: string): Promise<string> {
  const authorizedRoot = (await readFile(fixtureRootMarker, "utf8")).trim();
  const fixture = await mkdtemp(join(authorizedRoot, "case-"));
  await cp(resolve(repositoryRoot, "tests/fixtures/golden-rust-project"), fixture, {
    recursive: true,
  });
  await page.getByLabel("Server filesystem path").fill(fixture);
  await page.getByLabel("Project name").fill(name);
  await page.getByRole("button", { name: "Scan project" }).click();
  await expect(page.getByText(/files discovered for/)).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Import reviewed scan" }).click();
  await expect(page.getByRole("heading", { name: "Semantic" })).toBeVisible();
  return fixture;
}
