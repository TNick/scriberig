/** @file Provides browser diagnostics for the Scriberig E2E suite. */

import { expect, test as base, type Page } from "@playwright/test";

/** Browser-side failures collected during one test. */
export interface BrowserDiagnostics {
  consoleErrors: string[];
  pageErrors: string[];
  failedRequests: string[];
}

/** Installs strict browser diagnostics and returns the collected failures. */
export function installDiagnostics(page: Page): BrowserDiagnostics {
  const diagnostics: BrowserDiagnostics = {
    consoleErrors: [],
    pageErrors: [],
    failedRequests: [],
  };
  page.on("console", (message) => {
    if (message.type() === "error") diagnostics.consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => diagnostics.pageErrors.push(error.message));
  page.on("requestfailed", (request) => {
    if (!request.url().includes("favicon")) diagnostics.failedRequests.push(request.url());
  });
  return diagnostics;
}

/** Asserts that the browser completed the flow without runtime failures. */
export function expectHealthyBrowser(diagnostics: BrowserDiagnostics): void {
  expect(diagnostics.consoleErrors).toEqual([]);
  expect(diagnostics.pageErrors).toEqual([]);
  expect(diagnostics.failedRequests).toEqual([]);
}

/** Test fixture with the standard Scriberig browser diagnostics installed. */
export const test = base.extend<{ diagnostics: BrowserDiagnostics }>({
  diagnostics: async ({ page }, use) => {
    await use(installDiagnostics(page));
  },
});

export { expect };
