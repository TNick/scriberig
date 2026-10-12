/** @file Verifies the real browser golden project workflow. */

import { cp, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";
import type { Page } from "@playwright/test";

const repositoryRoot = resolve(process.cwd(), "..");
const fixtureRootMarker = resolve(repositoryRoot, "output/playwright/golden-fixture-root");

/** Creates an isolated copied Rust fixture below the server-authorized root.
 * @returns The copied fixture path.
 */
async function freshFixture(): Promise<string> {
  const authorizedRoot = (await readFile(fixtureRootMarker, "utf8")).trim();
  const fixture = await mkdtemp(join(authorizedRoot, "case-"));
  await cp(resolve(repositoryRoot, "tests/fixtures/golden-rust-project"), fixture, {
    recursive: true,
  });
  return fixture;
}

/** Imports one copied fixture and waits for the active semantic workspace.
 * @param page Browser page.
 * @param name Project display name.
 * @returns The copied fixture path.
 */
async function importFixture(page: Page, name: string): Promise<string> {
  const shell = new AppShell(page);
  await shell.open();
  await shell.navigate("projects");
  const fixture = await freshFixture();
  await page.getByLabel("Server filesystem path").fill(fixture);
  await page.getByLabel("Project name").fill(name);
  await page.getByRole("button", { name: "Scan project" }).click();
  await expect(page.getByText(/files discovered for/)).toBeVisible();
  await page.getByRole("button", { name: "Import reviewed scan" }).click();
  await expect(page.getByRole("heading", { name: "Semantic" })).toBeVisible();
  return fixture;
}

/** Verifies browser diagnostics while allowing the expected project rejection. */
function expectExpectedProjectError(
  diagnostics: {
    consoleErrors: string[];
    pageErrors: string[];
    failedRequests: string[];
    httpErrors: string[];
    privateRequests: string[];
  },
  status: string,
  expectedCount = 1,
): void {
  expect(diagnostics.consoleErrors).toHaveLength(expectedCount);
  expect(diagnostics.consoleErrors.every((error) => error.includes(`status of ${status}`))).toBe(
    true,
  );
  expect(diagnostics.pageErrors).toEqual([]);
  expect(diagnostics.failedRequests).toEqual([]);
  expect(diagnostics.privateRequests).toEqual([]);
  expect(diagnostics.httpErrors).toHaveLength(expectedCount);
  expect(
    diagnostics.httpErrors.every((error) =>
      error.includes(`${status} http://127.0.0.1:5173/api/protocol/project`),
    ),
  ).toBe(true);
}

test("WEB-GOLDEN-01 onboards and visibly navigates named Rust symbols", async ({
  page,
  diagnostics,
}) => {
  await importFixture(page, "golden-web-01");
  await expect(page.getByRole("list", { name: "Semantic project symbols" })).toContainText(
    "Calculator",
  );
  await expect(page.getByRole("list", { name: "Semantic project symbols" })).toContainText("add");
  await expect(page.getByRole("list", { name: "Semantic project symbols" })).toContainText(
    "src/lib.rs",
  );
  expectHealthyBrowser(diagnostics);
});

test("WEB-GOLDEN-02 selects add and shows actual Rust source", async ({ page, diagnostics }) => {
  await importFixture(page, "golden-web-02");
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  await expect(page.getByRole("region", { name: "Rust source projection" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Rust source projection" })).toContainText(
    "src/lib.rs",
  );
  await expect(page.getByRole("textbox", { name: "Source projection" })).toHaveValue(
    /left \+ right/,
  );
  expectHealthyBrowser(diagnostics);
});

test("WEB-GOLDEN-03 reviews and validates a candidate without write-through", async ({
  page,
  diagnostics,
}) => {
  const fixture = await importFixture(page, "golden-web-03");
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  const source = page.getByRole("textbox", { name: "Source projection" });
  const before = await source.inputValue();
  await source.fill(before.replace("left + right", "left.saturating_add(right)"));
  await page.getByRole("button", { name: "Review change" }).click();
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText(
    "left.saturating_add(right)",
  );
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText(
    "Validation: passed",
  );
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText("src/lib.rs");
  await expect(source).toHaveValue(/left\.saturating_add\(right\)/);
  await expect(readFile(join(fixture, "src", "lib.rs"), "utf8")).resolves.toContain("left + right");
  expectHealthyBrowser(diagnostics);
});

test("WEB-GOLDEN-04 applies, refreshes, shows history, and survives reload", async ({
  page,
  diagnostics,
}) => {
  const fixture = await importFixture(page, "golden-web-04");
  const before = await page
    .getByText(/snapshot snp_/)
    .first()
    .textContent();
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  const source = page.getByRole("textbox", { name: "Source projection" });
  await source.fill(
    (await source.inputValue()).replace("left + right", "left.saturating_add(right)"),
  );
  await page.getByRole("button", { name: "Review change" }).click();
  await page.getByRole("button", { name: "Apply migration" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Applied; successor snapshot" }),
  ).toBeVisible();
  await expect(readFile(join(fixture, "src", "lib.rs"), "utf8")).resolves.toContain(
    "left.saturating_add(right)",
  );
  await expect(page.getByText(/snapshot snp_/).first()).not.toHaveText(before ?? "");
  const historyResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/protocol/project") &&
      response.request().postData()?.includes('"history"'),
  );
  await page.getByRole("button", { name: "history", exact: true }).click();
  await expect((await historyResponse).status()).toBe(200);
  await expect(page.getByRole("region", { name: "History" }).first()).toContainText("snapshot");
  await page.reload();
  await page.getByRole("button", { name: "semantic", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Semantic" })).toBeVisible();
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Source projection" })).toHaveValue(
    /left\.saturating_add\(right\)/,
  );
  expectHealthyBrowser(diagnostics);
});

test("WEB-GOLDEN-05 invalid edit fails validation without mutation", async ({
  page,
  diagnostics,
}) => {
  await importFixture(page, "golden-web-05");
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  const source = page.getByRole("textbox", { name: "Source projection" });
  const original = await source.inputValue();
  await source.fill(original.replace("left + right", "left +"));
  await page.getByRole("button", { name: "Review change" }).click();
  await expect(page.getByText(/ServerProtocolError: unexpected end of input/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply migration" })).toHaveCount(0);
  await source.fill(original.replace("left + right", "left + unknown"));
  await page.getByRole("button", { name: "Review change" }).click();
  await expect(page.getByRole("status").filter({ hasText: /error\[E0425\]/ })).toBeVisible();
  await expect(page.getByRole("button", { name: "Apply migration" })).toHaveCount(0);
  expectExpectedProjectError(diagnostics, "400", 2);
});

test("WEB-GOLDEN-06 external source edits remain recoverable", async ({ page, diagnostics }) => {
  const fixture = await importFixture(page, "golden-web-06");
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "projection", exact: true }).click();
  const source = page.getByRole("textbox", { name: "Source projection" });
  await source.fill(
    (await source.inputValue()).replace("left + right", "left.saturating_add(right)"),
  );
  await page.getByRole("button", { name: "Review change" }).click();
  const file = join(fixture, "src", "lib.rs");
  await writeFile(file, `${await readFile(file, "utf8")}\n// external edit\n`);
  await page.getByRole("button", { name: "Apply migration" }).click();
  await expect(page.getByRole("status").filter({ hasText: "ServerProtocolError" })).toContainText(
    "source changed externally",
  );
  await expect(readFile(file, "utf8")).resolves.toContain("// external edit");
  expectExpectedProjectError(diagnostics, "409");
});

test("WEB-GOLDEN-07 agent uses semantic scope and enters normal review", async ({
  page,
  diagnostics,
}) => {
  const fixture = await importFixture(page, "golden-web-07");
  await page.getByRole("button", { name: /add · rust\.method/ }).click();
  await page.getByRole("button", { name: "session", exact: true }).click();
  const selection = await page.getByText(/^Selection: ent_/).textContent();
  const entity = selection?.match(/ent_[a-f0-9]+/)?.[0];
  expect(entity).toBeDefined();
  const workspace = await page
    .getByText(/connected \| project: .* \| snapshot: snp_/)
    .textContent();
  const snapshot = workspace?.match(/snapshot: (snp_[a-f0-9]+)/)?.[1];
  expect(snapshot).toBeDefined();
  await page.getByLabel("Prompt").fill("Change add so overflow saturates instead of wrapping.");
  const agentResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/protocol/project") &&
      response.request().postData()?.includes('"agent"'),
  );
  await page.getByRole("button", { name: "Start agent" }).click();
  const agentBody = await (await agentResponse).json();
  expect(agentBody.scope.entity).toBe(entity);
  expect(agentBody.scope.snapshot).toBe(snapshot);
  await expect(page.getByRole("button", { name: "projection", exact: true })).toBeVisible();
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText("src/lib.rs");
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText(
    "left + right",
  );
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText(
    "left.saturating_add(right)",
  );
  await expect(page.getByRole("region", { name: "Migration review" })).toContainText(
    "Validation: passed",
  );
  await expect(page.getByRole("textbox", { name: "Source projection" })).toHaveValue(
    /left \+ right/,
  );
  await expect(readFile(join(fixture, "src", "lib.rs"), "utf8")).resolves.toContain("left + right");
  expectHealthyBrowser(diagnostics);
});
