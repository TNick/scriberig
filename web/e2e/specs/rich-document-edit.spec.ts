/** @file Verifies a real browser creates and validates a DOCX paragraph candidate. */

import { cp, mkdtemp, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import paragraphEdit from "../../../tests/fixtures/document-paragraph-edit.json";
import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";

const repositoryRoot = resolve(process.cwd(), "..");
const fixtureRootMarker = resolve(repositoryRoot, "output/playwright/golden-fixture-root");

/** Imports an isolated copy of the shared rich project through the browser UI.
 * @param page Browser page displaying the application.
 * @returns Copy of the imported fixture used by this test.
 */
async function importRichProject(page: import("@playwright/test").Page): Promise<string> {
  const authorizedRoot = (await readFile(fixtureRootMarker, "utf8")).trim();
  const fixture = await mkdtemp(join(authorizedRoot, "rich-docx-"));
  await cp(resolve(repositoryRoot, "tests/fixtures/rich-project"), fixture, {
    recursive: true,
  });
  const shell = new AppShell(page);
  await shell.open();
  await shell.navigate("projects");
  await page.getByLabel("Server filesystem path").fill(fixture);
  await page.getByLabel("Project name").fill("rich-docx-browser-candidate");
  await page.getByRole("button", { name: "Scan project" }).click();
  await expect(page.getByText(/files discovered for/)).toBeVisible({ timeout: 30_000 });
  await page.getByRole("button", { name: "Import reviewed scan" }).click();
  await expect(page.getByRole("heading", { name: "Semantic" })).toBeVisible();
  return fixture;
}

test("WEB-RICH-DOCX-01 creates and validates the shared paragraph fixture candidate", async ({
  page,
  diagnostics,
}) => {
  const fixture = await importRichProject(page);
  const sourcePath = join(fixture, paragraphEdit.path);
  const original = await readFile(sourcePath);
  await page.getByRole("button", { name: /document\.root/ }).click();

  const document = page.getByRole("region", { name: "Semantic document" });
  const targetParagraph = document
    .locator('[aria-label="Document paragraph"]')
    .filter({ hasText: paragraphEdit.paragraphText });
  await expect(targetParagraph).toBeVisible();
  await targetParagraph
    .getByRole("textbox", { name: "Replacement paragraph text" })
    .fill(paragraphEdit.replacement);
  await targetParagraph.getByRole("button", { name: "Create edit candidate" }).click();
  await expect(document.getByRole("status")).toContainText("for report.docx is pending");

  const review = page.getByRole("region", { name: "Project candidate review" });
  await expect(review).toBeVisible();
  await expect(review).toContainText("report.docx");
  await review.getByRole("button", { name: "Validate candidate" }).click();
  await expect(review).toContainText("passed", { timeout: 30_000 });
  await expect(readFile(sourcePath)).resolves.toEqual(original);
  expectHealthyBrowser(diagnostics);
});
