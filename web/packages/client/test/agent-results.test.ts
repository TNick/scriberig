/** @file Verifies durable agent-result and operation response normalization. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
describe("durable agent results", () => {
  it("normalizes durable agent results from operation reads", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          reference: { id: "op-agent" },
          kind: "Agent",
          status: "Completed",
          progress: null,
          result_snapshot: null,
          result_handles: [],
          agent_result: {
            session: { id: "session-1" },
            root_branch: { id: "branch-1" },
            final_output: "done",
            branch_result: "result-1",
            produced_changes: [{ id: "ent-1" }],
            validation_results: [],
            unresolved_questions: [],
            candidate_changes: [
              {
                migration: "migration-1",
                entity: { id: "ent-1" },
                candidate_snapshot: { id: "snapshot-2" },
                validation: ["validation-1"],
              },
            ],
          },
          error: null,
        }) as T,
      subscribe: () => () => undefined,
    };
    const result = await new ScriberigClient(transport).getAgentResult("op-agent");
    expect(result).toMatchObject({
      sessionId: "session-1",
      rootBranchId: "branch-1",
      branchResultId: "result-1",
      finalOutput: "done",
      handles: ["ent-1"],
      candidateChanges: [
        {
          migrationId: "migration-1",
          entityId: "ent-1",
          candidateSnapshotId: "snapshot-2",
          validationRefs: ["validation-1"],
        },
      ],
    });
  });

  it("normalizes Rust operation status response wrappers", async () => {
    const operation = await new ScriberigClient({
      request: async <T>(): Promise<T> =>
        ({
          operation: {
            reference: { id: "op-agent" },
            kind: "Agent",
            status: "Completed",
            progress: null,
            result_snapshot: null,
            result_handles: [],
            agent_result: {
              session: { id: "session-1" },
              root_branch: { id: "branch-1" },
              branch_result: "result-1",
              final_output: "2",
              produced_changes: [],
              validation_results: [],
              unresolved_questions: [],
              candidate_changes: [],
            },
          },
        }) as T,
      subscribe: () => () => undefined,
    }).operation("op-agent");

    expect(operation).toMatchObject({
      operationId: "op-agent",
      status: "completed",
      result: { finalOutput: "2" },
    });
  });
});
