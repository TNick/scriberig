/** @file Covers bootstrap and every currently routed application surface. */

import { AppShell } from "../pages/app-shell";
import { expect, expectHealthyBrowser, test } from "../fixtures/app";

test("boots into the connected current raw knowledge view", async ({ page, diagnostics }) => {
  const shell = new AppShell(page);
  await shell.open();

  await expect(shell.snapshotStatus()).toHaveText("View state: current");
  await expect(page.getByRole("tree")).toBeVisible();
  await expect(page.getByRole("button", { name: "Expand" })).toBeVisible();
  await expect(page.getByText("No knowledge tree is connected.")).toHaveCount(0);
  expectHealthyBrowser(diagnostics);
});

test("navigates across every application surface without losing connection state", async ({
  page,
  diagnostics,
}) => {
  const shell = new AppShell(page);
  await shell.open();

  const surfaces = [
    ["semantic", "Semantic", "semantic domain views"],
    ["projection", "Projection", "projection execution"],
    ["history", "History / Diff", "history and semantic diff"],
    ["session", "Session / Agent", "sessions and agent operations"],
    ["operations", "Operations", "long-running operations"],
  ] as const;
  for (const [, heading, capability] of surfaces) {
    await shell.navigate(heading.split(" /")[0].toLowerCase());
    await expect(page.getByRole("heading", { name: heading }).first()).toBeVisible();
    await expect(page.getByText(`Unsupported capability: ${capability}`)).toBeVisible();
    await expect(page.getByRole("status").first()).toHaveText("connected");
  }
  expectHealthyBrowser(diagnostics);
});
