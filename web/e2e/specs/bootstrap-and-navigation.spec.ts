/** @file Covers bootstrap and every currently routed application surface. */

import { AppShell } from "../pages/app-shell";
import { expect, expectHealthyBrowser, test } from "../fixtures/app";

test("boots into the connected project catalog", async ({ page, diagnostics }) => {
  const shell = new AppShell(page);
  await shell.open();

  await expect(shell.snapshotStatus()).toHaveText("View state: current");
  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
  await expect(page.getByLabel("Server filesystem path")).toBeVisible();
  expectHealthyBrowser(diagnostics);
});

test("navigates across every application surface without losing connection state", async ({
  page,
  diagnostics,
}) => {
  const shell = new AppShell(page);
  await shell.open();

  const surfaces = [
    ["semantic", "Semantic"],
    ["projection", "Projection"],
    ["history", "History / Diff"],
    ["session", "Session / Agent"],
    ["migration", "Migration Review"],
    ["operations", "Operations"],
  ] as const;
  for (const [route, heading] of surfaces) {
    await shell.navigate(route);
    await expect(page.locator("h1")).toHaveText(heading);
    await expect(page.getByText(/Unsupported capability:/)).toHaveCount(0);
    await expect(page.getByRole("status").first()).toHaveText("connected");
  }
  expectHealthyBrowser(diagnostics);
});
