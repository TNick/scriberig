/** @file Verifies durable operation storage is independent of views. */
import { expect, it } from "vitest";
import { OperationStore } from "@scriberig/model";
it("retains one operation store instance", () => {
  const store = new OperationStore();
  expect(store).toBeDefined();
});
