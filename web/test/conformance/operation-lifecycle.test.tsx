/** @file Verifies durable operation storage is independent of views. */
import { expect, it } from "vitest";
import { OperationStore } from "@scriberig/model";
it("retains terminal operation state after a view unsubscribes", () => {
  const store = new OperationStore();
  const operationId = "operation-1" as never;
  store.set({ operationId, kind: "agent", status: "queued" });
  const unsubscribe = store.subscribe(() => undefined);
  unsubscribe();
  store.complete(operationId);
  expect(store.get(operationId)).toMatchObject({ status: "completed" });
});
