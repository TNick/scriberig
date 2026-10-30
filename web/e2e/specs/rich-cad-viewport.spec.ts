/** @file Verifies CAD pan and zoom remain presentation-only. */

import { cp, mkdtemp, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";

const repositoryRoot = resolve(process.cwd(), "..");
const fixtureRootMarker = resolve(repositoryRoot, "output/playwright/golden-fixture-root");

/** Imports a fresh mixed rich project through the public product workflow.
 * @param page Browser page displaying the application.
 * @returns Completion after the semantic project view is ready.
 */
async function importRichProject(page: import("@playwright/test").Page): Promise<void> {
  const authorizedRoot = (await readFile(fixtureRootMarker, "utf8")).trim();
  const fixture = await mkdtemp(join(authorizedRoot, "rich-cad-viewport-"));
  await cp(resolve(repositoryRoot, "tests/fixtures/rich-project"), fixture, {
    recursive: true,
  });
  const shell = new AppShell(page);
  await shell.open();
  await shell.navigate("projects");
  await page.getByLabel("Server filesystem path").fill(fixture);
  await page.getByLabel("Project name").fill("rich-cad-viewport");
  await page.getByRole("button", { name: "Scan project" }).click();
  await expect(page.getByText(/files discovered for/)).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Import reviewed scan" }).click();
  await expect(page.getByRole("heading", { name: "Semantic" })).toBeVisible();
}

test("WEB-RICH-CAD-VIEWPORT-01 pans and zooms without changing semantic state", async ({
  page,
  diagnostics,
}) => {
  const protocolRequests: string[] = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && request.url().includes("/api/protocol/")) {
      protocolRequests.push(request.url());
    }
  });
  await importRichProject(page);
  await page.getByRole("button", { name: /cad\.drawing/ }).click();

  const drawing = page.getByRole("region", { name: "Semantic CAD drawing" });
  const preview = drawing.getByRole("group", { name: "Semantic CAD entities" });
  await expect(preview).toBeVisible({ timeout: 30_000 });
  await expect(preview.locator("[data-entity-id]").first()).toHaveAttribute(
    "aria-label",
    /^Select CAD entity ent_/,
  );
  const selection = page.getByText(/^Entity ent_/);
  await expect(selection).toBeVisible();
  const selectedEntity = await selection.textContent();
  const requestCount = protocolRequests.length;
  const originalViewBox = await preview.getAttribute("viewBox");

  await drawing.getByRole("button", { name: "Zoom in", exact: true }).click();
  await expect(preview).not.toHaveAttribute("viewBox", originalViewBox ?? "");
  const zoomedViewBox = await preview.getAttribute("viewBox");
  await drawing.getByRole("button", { name: "Pan right", exact: true }).click();
  await expect(preview).not.toHaveAttribute("viewBox", zoomedViewBox ?? "");
  await expect(selection).toHaveText(selectedEntity ?? "");
  expect(protocolRequests).toHaveLength(requestCount);
  expectHealthyBrowser(diagnostics);
});
