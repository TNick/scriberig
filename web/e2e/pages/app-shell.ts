/** @file Models the user-visible Scriberig application shell. */

import { expect, type Page } from "@playwright/test";

/** Provides stable interactions with top-level application navigation. */
export class AppShell {
  public constructor(private readonly page: Page) {}

  /** Opens the application and waits for the connected root view. */
  public async open(): Promise<void> {
    await this.page.goto("/");
    await expect(this.page).toHaveTitle("Scriberig");
    await expect(this.page.getByRole("status").first()).toHaveText("connected");
    await expect(this.page.getByRole("heading", { name: "Raw Knowledge" })).toBeVisible();
  }

  /** Navigates to a top-level view by its accessible navigation name. */
  public async navigate(name: string): Promise<void> {
    await this.page.getByRole("button", { name, exact: true }).click();
  }

  /** Returns the snapshot status text shown by the active view. */
  public snapshotStatus() {
    return this.page.getByText(/View state:/);
  }
}
