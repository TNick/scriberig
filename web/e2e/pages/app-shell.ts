/** @file Models the user-visible Scriberig application shell. */

import { expect, type Page } from "@playwright/test";

/** Provides stable interactions with top-level application navigation. */
/**
 * Public members: `open`, `navigate`, and `snapshotStatus`.
 * Private members: `page`.
 */
export class AppShell {
  /** Creates an application shell page object.
   * @param page Playwright page to control.
   */
  public constructor(private readonly page: Page) {}

  /** Opens the application and waits for the connected project catalog. */
  /** @returns Resolves when the projects view is visible. */
  public async open(): Promise<void> {
    await this.page.goto("/");
    await expect(this.page).toHaveTitle("Scriberig");
    await expect(this.page.getByRole("status").first()).toHaveText("connected");
    await expect(this.page.getByRole("heading", { name: "Projects" })).toBeVisible();
  }

  /** Navigates to a top-level view by its accessible navigation name. */
  /** @param name Accessible navigation label to activate. */
  /**
   * @param name Accessible navigation label to activate.
   * @returns Resolves when navigation completes.
   */
  public async navigate(name: string): Promise<void> {
    await this.page.getByRole("button", { name, exact: true }).click();
  }

  /** Returns the snapshot status text shown by the active view. */
  /** @returns Locator for the active snapshot status. */
  public snapshotStatus() {
    return this.page.getByText(/View state:/);
  }
}
