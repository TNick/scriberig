/** @file Observes durable operation state through the model operation store. */
import { useCallback, useSyncExternalStore } from "react";
import type { Operation, OperationId } from "@scriberig/protocol";
import { useScriberig } from "./provider";

/** Observes one operation independently of component lifetime.
 * @param operationId Operation identity.
 * @returns Operation state and cancellation availability.
 */
export function useOperation(operationId?: OperationId): {
  operation: Operation | undefined;
  cancel?: () => void;
} {
  const { model } = useScriberig();
  const subscribe = useCallback(
    (listener: () => void) => model.operations.subscribe(listener),
    [model],
  );
  const operation = useSyncExternalStore(
    subscribe,
    () => (operationId ? model.operations.get(operationId) : undefined),
    () => (operationId ? model.operations.get(operationId) : undefined),
  );
  return { operation };
}
