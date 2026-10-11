/** @file Tracks durable long-running protocol operations. */
import type { Operation, OperationId } from "@scriberig/protocol";
/** Stores operation lifecycle records independently of UI components.
 *
 * Public members: `set`, `get`, `all`, `progress`, `fail`, `cancel`,
 * `output`, and `subscribe`.
 * Private members: the internal operation map and listeners.
 */
export class OperationStore {
  private readonly values = new Map<OperationId, Operation>();
  private readonly listeners = new Set<() => void>();
  private snapshot: readonly Operation[] = [];
  /** Inserts or replaces an operation record. @param operation Operation record. */
  set(operation: Operation): void {
    this.values.set(operation.operationId, operation);
    this.snapshot = [...this.values.values()];
    for (const listener of this.listeners) listener();
  }
  /** Reads an operation. @param operationId Operation identity. @returns Operation record. */
  get(operationId: OperationId): Operation | undefined {
    return this.values.get(operationId);
  }
  /** Lists operation records in insertion order. @returns Operation records. */
  all(): readonly Operation[] {
    return this.snapshot;
  }
  /** Applies progress idempotently. @param operationId Operation identity. @param progress Progress value. */
  progress(operationId: OperationId, progress: number): void {
    this.update(operationId, (old) => ({
      ...old,
      status: "running",
      progress,
    }));
  }
  /** Marks an operation as failed. @param operationId Operation identity. @param error Failure detail. */
  fail(operationId: OperationId, error: string): void {
    this.update(operationId, (old) => ({ ...old, status: "failed", error }));
  }
  /** Marks an operation as cancelled. @param operationId Operation identity. */
  cancel(operationId: OperationId): void {
    this.update(operationId, (old) => ({ ...old, status: "cancelled" }));
  }
  /** Appends agent output to an operation. @param operationId Operation identity. @param text Output chunk. */
  output(operationId: OperationId, text: string): void {
    this.update(operationId, (old) => ({
      ...old,
      status: "running",
      output: `${old.output ?? ""}${text}`,
    }));
  }
  /** Updates an operation, creating a placeholder for an early event.
   * @param operationId Operation identity.
   * @param update State transformation to apply.
   */
  private update(operationId: OperationId, update: (operation: Operation) => Operation): void {
    const current = this.values.get(operationId) ?? {
      operationId,
      kind: "unknown",
      status: "queued",
    };
    this.values.set(operationId, update(current));
    this.snapshot = [...this.values.values()];
    for (const listener of this.listeners) listener();
  }
  /** Subscribes to operation changes. @param listener Change listener. @returns Unsubscribe function. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}
