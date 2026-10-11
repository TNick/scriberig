/** @file Defines snapshot-aware headless navigation history. */
import type { Selection } from "./selection";
/** Back/forward navigation controller preserving exact selections.
 *
 * Public members: `open`, `currentSelection`, `back`, and `forward`.
 * Private members: current selection and navigation stacks.
 */
export class Navigation {
  private backStack: Selection[] = [];
  private forwardStack: Selection[] = [];
  private current: Selection | undefined;
  /** Opens a selection. @param selection Selection to open. */
  open(selection: Selection): void {
    if (this.current) this.backStack.push(this.current);
    this.current = selection;
    this.forwardStack = [];
  }
  /** Reads current selection. @returns Current selection. */
  currentSelection(): Selection | undefined {
    return this.current;
  }
  /** Navigates backward. @returns Previous selection. */
  back(): Selection | undefined {
    if (!this.backStack.length) return;
    if (this.current) this.forwardStack.push(this.current);
    this.current = this.backStack.pop();
    return this.current;
  }
  /** Navigates forward. @returns Next selection. */
  forward(): Selection | undefined {
    if (!this.forwardStack.length) return;
    if (this.current) this.backStack.push(this.current);
    this.current = this.forwardStack.pop();
    return this.current;
  }
}
