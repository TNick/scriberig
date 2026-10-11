/** @file Covers lazy knowledge browsing and semantic selection behavior. */

import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";

test("loads children only after expanding the root knowledge node", async ({
  page,
  diagnostics,
}) => {
  const listRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().endsWith("/api/protocol/list")) listRequests.push(request.postData() ?? "");
  });
  const shell = new AppShell(page);
  await shell.open();

  expect(listRequests).toEqual([]);
  await page.getByRole("button", { name: "Expand" }).click();
  await expect(page.getByRole("button", { name: "Collapse" })).toBeVisible();
  await expect(page.getByRole("tree").getByRole("listitem")).toHaveCount(3);
  expect(listRequests).toHaveLength(1);
  expect(listRequests[0]).toContain('"relation":"children"');
  expectHealthyBrowser(diagnostics);
});

test("preserves the selected semantic identity across supported views", async ({
  page,
  diagnostics,
}) => {
  const shell = new AppShell(page);
  await shell.open();
  await page.getByRole("button", { name: "Expand" }).click();
  const child = page.getByRole("tree").getByRole("listitem").nth(1);
  await child.getByRole("button").first().click();

  await shell.navigate("semantic");
  await expect(page.getByText(/Entity ent_/)).toBeVisible();
  await shell.navigate("raw");
  await shell.navigate("semantic");
  await expect(page.getByText(/Entity ent_/)).toBeVisible();
  expectHealthyBrowser(diagnostics);
});
