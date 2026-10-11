/** @file Tracks durable long-running protocol operations. */
import type { Operation, OperationId } from "@scriberig/protocol";
/** Stores operation lifecycle records independently of UI components.
 *
 * Public members: `set`, `get`, `progress`, and `subscribe`.
 * Private members: the internal operation map and listeners.
 */
export class OperationStore {
  private readonly values = new Map<OperationId, Operation>();
  private readonly listeners = new Set<() => void>();
  /** Inserts or replaces an operation record. @param operation Operation record. */
  set(operation: Operation): void {
    this.values.set(operation.operationId, operation);
    for (const listener of this.listeners) listener();
  }
  /** Reads an operation. @param operationId Operation identity. @returns Operation record. */
  get(operationId: OperationId): Operation | undefined {
    return this.values.get(operationId);
  }
  /** Applies progress idempotently. @param operationId Operation identity. @param progress Progress value. */
  progress(operationId: OperationId, progress: number): void {
    const old = this.values.get(operationId);
    if (old) {
      this.values.set(operationId, { ...old, progress });
      for (const listener of this.listeners) listener();
    }
  }
  /** Subscribes to operation changes. @param listener Change listener. @returns Unsubscribe function. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}
