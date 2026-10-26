/** @file Covers explicit disconnected behavior at the browser boundary. */

import { expect, expectHealthyBrowser, test } from "../fixtures/app";
test("reports disconnected state when bootstrap requests cannot reach the API", async ({
  page,
  diagnostics,
}) => {
  await page.route("**/api/**", (route) => route.abort("failed"));
  await page.goto("/");

  await expect(page.getByRole("status").first()).toHaveText("disconnected");
  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
  await expect(page.getByRole("alert")).toContainText("TransportError");
  diagnostics.consoleErrors.length = 0;
  expectHealthyBrowser({
    ...diagnostics,
    failedRequests: [],
  });
});

test("returns to connected state after a failed bootstrap is retried by reload", async ({
  page,
  diagnostics,
}) => {
  let blocked = true;
  await page.route("**/api/**", async (route) => {
    if (blocked) {
      await route.abort("failed");
      return;
    }
    await route.continue();
  });
  await page.goto("/");
  await expect(page.getByRole("status").first()).toHaveText("disconnected");
  diagnostics.consoleErrors.length = 0;
  diagnostics.failedRequests.length = 0;
  blocked = false;
  await page.reload();
  await expect(page.getByRole("status").first()).toHaveText("connected");
  await expect(page.getByRole("heading", { name: "Projects" })).toBeVisible();
  await expect(page.getByRole("list", { name: "Projects" })).toBeVisible();
  expectHealthyBrowser({
    ...diagnostics,
    failedRequests: [],
  });
});
