/** @file Covers browser project scan and import onboarding. */

import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";
import { cp, mkdtemp, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repositoryRoot = resolve(process.cwd(), "..");
const fixtureRootMarker = resolve(repositoryRoot, "output/playwright/golden-fixture-root");

test("scans and imports a project through the Projects screen", async ({ page, diagnostics }) => {
  const shell = new AppShell(page);
  await shell.open();
  await shell.navigate("projects");

  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
  const authorizedRoot = (await readFile(fixtureRootMarker, "utf8")).trim();
  const fixture = await mkdtemp(join(authorizedRoot, "onboarding-"));
  await cp(resolve(repositoryRoot, "tests/fixtures/golden-rust-project"), fixture, {
    recursive: true,
  });
  await page.getByLabel("Server filesystem path").fill(fixture);
  await page.getByLabel("Project name").fill("scriberig-web-e2e");
  await page.getByRole("button", { name: "Scan project" }).click();

  await expect(page.getByText(/files discovered for/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Import reviewed scan" })).toBeVisible();
  const importResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/protocol/project") &&
      response.request().postData()?.includes('"import"') === true,
  );
  await page.getByRole("button", { name: "Import reviewed scan" }).click();
  const response = await importResponse;
  expect(response.ok()).toBe(true);
  const body = (await response.json()) as {
    response?: { Project?: { result?: { project?: { status?: string } } } };
  };
  const imported = body.response?.Project?.result;
  expect(imported.project?.status).toBe("imported");

  await expect(page.getByRole("list", { name: "Projects" })).toContainText("imported");
  expectHealthyBrowser(diagnostics);
});
