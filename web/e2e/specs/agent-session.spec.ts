/** @file Verifies browser agent execution through the real Scriberig API. */

import { expect, expectHealthyBrowser, test } from "../fixtures/app";
import { AppShell } from "../pages/app-shell";

/** Runs a browser agent request through the managed API and checks durable completion. */
test("WEB-AGENT-01 starts a valid session and receives its provider result", async ({
  page,
  diagnostics,
}) => {
  const shell = new AppShell(page);
  await shell.open();
  await shell.navigate("session");

  const startRequest = page.waitForRequest(
    (request) => request.method() === "POST" && request.url().endsWith("/api/protocol/agent_start"),
  );
  await page.getByRole("textbox", { name: "Prompt" }).fill("Return the deterministic answer.");
  await page.getByRole("button", { name: "Start agent" }).click();

  const request = await startRequest;
  const payload = request.postDataJSON() as { session?: { id?: unknown }; prompt?: unknown };
  expect(payload.prompt).toBe("Return the deterministic answer.");
  expect(payload.session?.id).toMatch(/^ent_[a-f0-9]{32}$/);
  expect(Object.keys(payload).sort()).toEqual(["prompt", "session"]);
  expect(Object.keys(payload.session ?? {})).toEqual(["id"]);

  const operation = page.getByRole("region", { name: /^Agent operation / });
  await expect(operation).toContainText(/Status: completed/i, { timeout: 30_000 });
  await expect(operation).toContainText("Result: 2");
  expectHealthyBrowser(diagnostics);
});
