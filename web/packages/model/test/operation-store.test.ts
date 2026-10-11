/** @file Tests operation records exposed independently of views. */

import { describe, expect, it } from "vitest";
import type { Operation } from "@scriberig/protocol";
import { OperationStore } from "../src";

const operation: Operation = {
  operationId: "operation-1" as never,
  kind: "migration",
  status: "running",
  progress: 0.5,
};

describe("OperationStore", () => {
  it("lists durable operation records in insertion order", () => {
    const store = new OperationStore();
    store.set(operation);

    expect(store.all()).toEqual([operation]);
  });
});
