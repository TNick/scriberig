/** @file Provides framework-independent model subscriptions. */
/** A callback invoked after a consistent model transition. */
export type Listener<T> = (value: T) => void;
/** Small subscribe/unsubscribe registry.
 *
 * Public members: `subscribe` and `notify`.
 * Private members: the listener set.
 */
export class Subscription<T> {
  private readonly listeners = new Set<Listener<T>>();
  /** Adds a listener. @param listener Callback. @returns Unsubscribe function. */
  subscribe(listener: Listener<T>): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  /** Notifies all current listeners. @param value Consistent model value. */
  notify(value: T): void {
    for (const listener of this.listeners) listener(value);
  }
}
