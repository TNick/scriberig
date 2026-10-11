/** @file Provides browser diagnostics for the Scriberig E2E suite. */

import { expect, test as base, type Page } from "@playwright/test";

/** Browser-side failures collected during one test. */
export interface BrowserDiagnostics {
  consoleErrors: string[];
  pageErrors: string[];
  failedRequests: string[];
  privateRequests: string[];
}

/** Installs strict browser diagnostics and returns the collected failures. */
/**
 * @param page Browser page whose events should be observed.
 * @returns Diagnostics collected from the browser page.
 */
export function installDiagnostics(page: Page): BrowserDiagnostics {
  const diagnostics: BrowserDiagnostics = {
    consoleErrors: [],
    pageErrors: [],
    failedRequests: [],
    privateRequests: [],
  };
  page.on("console", (message) => {
    if (message.type() === "error") diagnostics.consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => diagnostics.pageErrors.push(error.message));
  page.on("requestfailed", (request) => {
    const failure = request.failure()?.errorText ?? "";
    if (!request.url().includes("favicon") && !failure.includes("ERR_ABORTED")) {
      diagnostics.failedRequests.push(request.url());
    }
  });
  page.on("request", (request) => {
    const url = request.url().toLowerCase();
    if (/(sqlite|database|crates|server\/|\.rs(?:$|\?))/.test(url)) {
      diagnostics.privateRequests.push(request.url());
    }
  });
  return diagnostics;
}

/** Asserts that the browser completed the flow without runtime failures. */
/** @param diagnostics Diagnostics collected during the browser flow. */
export function expectHealthyBrowser(diagnostics: BrowserDiagnostics): void {
  expect(diagnostics.consoleErrors).toEqual([]);
  expect(diagnostics.pageErrors).toEqual([]);
  expect(diagnostics.failedRequests).toEqual([]);
  expect(diagnostics.privateRequests).toEqual([]);
}

/** Test fixture with the standard Scriberig browser diagnostics installed. */
export const test = base.extend<{ diagnostics: BrowserDiagnostics }>({
  diagnostics: async ({ page }, use) => {
    await use(installDiagnostics(page));
  },
});

export { expect };
