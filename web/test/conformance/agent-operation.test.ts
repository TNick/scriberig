/** @file Verifies browser agent results against the shared server conformance fixture. */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { ScriberigClient } from "../../packages/client/src";

const fixture = JSON.parse(
  readFileSync(
    resolve(process.cwd(), "../tests/conformance/fixtures/agent-operation.json"),
    "utf8",
  ),
) as {
  request: { sessionId: string; prompt: string };
  accepted: unknown;
  operation: unknown;
  session: unknown;
  expected: {
    finalOutput: string;
    branchResultId: string;
    turnOrdinal: number;
    branchStatus: string;
  };
};

it("reads the accepted agent result and durable branch that Rust surfaces expose", async () => {
  const requests: Array<[string, unknown]> = [];
  const client = new ScriberigClient({
    request: async <T>(operation: string, payload: unknown): Promise<T> => {
      requests.push([operation, payload]);
      const response =
        operation === "agent_start"
          ? fixture.accepted
          : operation === "operation"
            ? fixture.operation
            : operation === "session"
              ? fixture.session
              : undefined;
      if (response === undefined) throw new Error(`unexpected operation: ${operation}`);
      return response as T;
    },
    subscribe: () => () => undefined,
  });

  const accepted = await client.startAgent({
    sessionId: fixture.request.sessionId as never,
    prompt: fixture.request.prompt,
  });
  const result = await client.getAgentResult(accepted.operationId);
  const session = await client.getSession(accepted.sessionId);
  const branch = session.branches.find((item) => item.id === accepted.branchId);

  expect(accepted).toEqual({
    operationId: "operation-agent-1",
    sessionId: fixture.request.sessionId,
    branchId: "ent_03030303030303030303030303030303",
  });
  expect(result).toMatchObject({
    sessionId: fixture.request.sessionId,
    rootBranchId: accepted.branchId,
    branchResultId: fixture.expected.branchResultId,
    finalOutput: fixture.expected.finalOutput,
  });
  expect(branch).toMatchObject({
    status: fixture.expected.branchStatus,
    result: { id: fixture.expected.branchResultId },
    turns: [
      {
        ordinal: fixture.expected.turnOrdinal,
        hasAgentOutput: true,
        agentOutput: fixture.expected.finalOutput,
      },
    ],
  });
  expect(requests).toEqual([
    ["agent_start", { session: { id: fixture.request.sessionId }, prompt: fixture.request.prompt }],
    ["operation", { operation: { id: "operation-agent-1" } }],
    ["session", { session: { id: fixture.request.sessionId } }],
  ]);
});
