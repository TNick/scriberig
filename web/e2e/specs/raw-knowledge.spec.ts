/** @file Covers lazy knowledge browsing and semantic selection behavior. */

import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { importGoldenProject } from "../fixtures/project";
import { AppShell } from "../pages/app-shell";

test("reveals project symbols only after expanding the root knowledge node", async ({
  page,
  diagnostics,
}) => {
  const listRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith("/api/protocol/list")) listRequests.push(request.postData() ?? "");
  });
  const shell = new AppShell(page);
  await shell.open();
  await importGoldenProject(page, "raw-knowledge-lazy-load");
  await shell.navigate("raw");

  expect(listRequests).toEqual([]);
  await page.getByRole("button", { name: "Expand" }).click();
  await expect(page.getByRole("button", { name: "Collapse" })).toBeVisible();
  await expect(page.getByRole("tree").getByRole("listitem")).toHaveCount(6);
  await expect(page.getByRole("tree")).toContainText("add · rust.method");
  expect(listRequests).toEqual([]);
  expectHealthyBrowser(diagnostics);
});

test("preserves the selected semantic identity across supported views", async ({
  page,
  diagnostics,
}) => {
  const shell = new AppShell(page);
  await shell.open();
  await importGoldenProject(page, "raw-knowledge-selection");
  await shell.navigate("raw");
  await page.getByRole("button", { name: "Expand" }).click();
  await page
    .getByRole("tree")
    .getByRole("button", { name: /add · rust\.method/ })
    .click();
  const selectedIdentity = await page.getByText(/^Entity ent_/).textContent();

  await shell.navigate("semantic");
  await expect(page.getByRole("region", { name: "Object inspector" })).toContainText("rust.method");
  await expect(page.getByText(/^Entity ent_/).first()).toHaveText(selectedIdentity ?? "");
  await shell.navigate("raw");
  await shell.navigate("semantic");
  await expect(page.getByText(/^Entity ent_/).first()).toHaveText(selectedIdentity ?? "");
  expectHealthyBrowser(diagnostics);
});
