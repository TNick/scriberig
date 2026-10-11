/** @file Tracks durable long-running protocol operations. */
import type { Operation, OperationId } from "@scriberig/protocol";
/** Stores operation lifecycle records independently of UI components.
 *
 * Public members: `set`, `get`, and `progress`.
 * Private members: the internal operation map.
 */
export class OperationStore {
  private readonly values = new Map<OperationId, Operation>();
  /** Inserts or replaces an operation record. @param operation Operation record. */
  set(operation: Operation): void {
    this.values.set(operation.operationId, operation);
  }
  /** Reads an operation. @param operationId Operation identity. @returns Operation record. */
  get(operationId: OperationId): Operation | undefined {
    return this.values.get(operationId);
  }
  /** Applies progress idempotently. @param operationId Operation identity. @param progress Progress value. */
  progress(operationId: OperationId, progress: number): void {
    const old = this.values.get(operationId);
    if (old) this.values.set(operationId, { ...old, progress });
  }
}
