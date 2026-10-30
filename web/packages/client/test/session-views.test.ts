/** @file Verifies Rust session reads and derived branch views. */
import { describe, expect, it } from "vitest";
import { ScriberigClient } from "../src";
import { normalizeSessionPlan } from "../src/session-plan-views";

describe("session views", () => {
  it("preserves shared semantic plan and node view fields", () => {
    const plan = normalizeSessionPlan({
      id: "plan-1",
      revision: 5,
      exact_revision: "plan-revision-5",
      snapshot: "snapshot-5",
      goal: "Ship the feature",
      status: "Running",
      root_branch: "branch-1",
      root_nodes: ["node-1"],
      status_counts: { planned: 0, ready: 0, running: 1, completed: 0 },
      nodes: [
        {
          id: "node-1",
          parent: "branch:branch-1",
          title: "Implement",
          purpose: "Implement the feature",
          status: "Running",
          exact_revision: "node-revision-2",
          effective_status: "Running",
          contains_ignored: true,
          summary_id: "summary-1",
          summary_preview: "Halfway done",
          dependency_blockers: ["node-0"],
          entry_gates: { pending: 1 },
          completion_gates: {},
          evidence_count: 2,
          can_retry: false,
        },
      ],
      dependencies: [],
      gates: [
        {
          id: "gate-1",
          owner: "node-1",
          phase: "Entry",
          requirement: { ValidationPassed: { profile: "lint" } },
        },
      ],
      evidence: [
        {
          id: "evidence-1",
          owner: { Gate: "gate-1" },
          role: "Supports",
          evidence: {
            kind: "ValidationResult",
            reference: { entity: "validation-1", revision: "revision-1" },
            attributes: { profile: "lint" },
          },
        },
      ],
      summaries: [
        {
          id: "summary-1",
          node: "node-1",
          text: "Halfway done",
          sources: [{ entity: "validation-1", revision: "revision-1" }],
        },
      ],
      history: [
        {
          revision: 4,
          goal: "Ship the feature",
          status: "Planned",
          nodes: [],
          dependencies: [],
          summaries: [{ id: "old-summary", node: "node-1", text: "Earlier" }],
        },
      ],
    });

    expect(plan).toMatchObject({
      exactRevision: "plan-revision-5",
      snapshot: "snapshot-5",
      rootNodes: ["node-1"],
      statusCounts: { running: 1, completed: 0 },
      gates: [{ id: "gate-1", requirement: { ValidationPassed: { profile: "lint" } } }],
      evidence: [{ id: "evidence-1", owner: { Gate: "gate-1" } }],
      summaries: [{ id: "summary-1", node: "node-1", text: "Halfway done" }],
      nodes: [
        {
          exactRevision: "node-revision-2",
          effectiveStatus: "Running",
          containsIgnored: true,
          summaryPreview: "Halfway done",
          dependencyBlockers: ["node-0"],
          entryGates: { pending: 1, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 2,
        },
      ],
      history: [{ summaries: [{ id: "old-summary", node: "node-1" }] }],
    });
  });

  it("normalizes Rust session reads and derives branch views", async () => {
    const transport = {
      request: async <T>(): Promise<T> =>
        ({
          session: { id: "session-1" },
          execution: {
            revision: 3,
            provider_id: "provider-1",
            model_id: "model-a",
            thinking_level: "careful",
            history: [
              {
                revision: 3,
                kind: "ThinkingLevelChanged",
                summary: "Thinking level selected: careful",
                provider_id: "provider-1",
                model_id: "model-a",
                thinking_level: "careful",
              },
            ],
          },
          branches: [
            {
              id: "root",
              title: "Root",
              status: "Completed",
              children: ["child"],
              turns: [{ id: "turn-1", ordinal: 1, has_human_input: true, has_agent_output: false }],
              activity: [
                {
                  id: "tool-call-1",
                  session: "session-1",
                  sequence: 2,
                  branch: "root",
                  kind: "ToolInvocation",
                  state: "Completed",
                  summary: "Tool: search",
                  tool_name: "search",
                  tool_arguments: { query: "architecture", token: "[REDACTED]" },
                },
                {
                  id: "tool-result-1",
                  session: "session-1",
                  sequence: 3,
                  branch: "root",
                  parent: "tool-call-1",
                  kind: "ToolResult",
                  state: "Completed",
                  summary: "Tool completed",
                  durable_ref: "tool-result-1",
                },
              ],
            },
            {
              id: "child",
              parent: "root",
              title: "Child",
              status: "Completed",
              children: [],
              turns: [],
              result: {
                id: "result-1",
                references: ["migration-1"],
                result_handles: ["artifact-1"],
              },
            },
          ],
          plans: [
            {
              id: "plan-1",
              revision: 4,
              goal: "Implement parser and tests",
              status: "Running",
              anchor: { BranchHead: { branch: "root" } },
              root_branch: "root",
              nodes: [
                {
                  id: "plan-parser",
                  parent: "root",
                  title: "Implement parser",
                  purpose: "Add semantic parser operations",
                  status: "Completed",
                  branch: "parser-branch",
                  result: "parser-result",
                  superseded_by: "parser-v2",
                  expected_result: { kind: "semantic_changes", requires_handle: true },
                  semantic_scope: ["entity-parser"],
                  assignment: {
                    provider_id: "provider-1",
                    model_id: "model-a",
                    thinking_level: "careful",
                  },
                  claim: {
                    id: "claim-parser",
                    node: "plan-parser",
                    branch: "parser-branch",
                    owner: "worker-1",
                    generation: 2,
                    assignment: { provider_id: "provider-1", model_id: "model-a" },
                  },
                },
              ],
              dependencies: [
                {
                  prerequisite: "plan-parser",
                  dependent: "plan-tests",
                  fulfilled_by: "parser-result",
                },
              ],
              coordinator: "root",
              coordinator_cursor: "plan-tests",
              coordinator_claim: {
                id: "coordinator-1",
                operation: "operation-1",
                owner: "runtime-1",
                generation: 3,
              },
              history: [
                {
                  revision: 3,
                  goal: "Earlier goal",
                  status: "Planned",
                  nodes: [],
                  dependencies: [],
                  reason: "Clarified scope",
                },
              ],
            },
          ],
        }) as T,
      subscribe: () => () => undefined,
    };
    const client = new ScriberigClient(transport);

    const session = await client.getSession("session-1" as never);
    expect(session.execution).toEqual({
      revision: 3,
      providerId: "provider-1",
      modelId: "model-a",
      thinkingLevel: "careful",
      history: [
        {
          revision: 3,
          kind: "ThinkingLevelChanged",
          summary: "Thinking level selected: careful",
          providerId: "provider-1",
          modelId: "model-a",
          thinkingLevel: "careful",
        },
      ],
    });
    expect(session.branches[0]?.turns[0]?.hasHumanInput).toBe(true);
    expect(session.plans[0]).toMatchObject({
      id: "plan-1",
      revision: 4,
      goal: "Implement parser and tests",
      status: "Running",
      coordinator: "root",
      anchor: { BranchHead: { branch: "root" } },
      coordinatorClaim: { id: "coordinator-1", generation: 3 },
      history: [{ revision: 3, goal: "Earlier goal", reason: "Clarified scope" }],
      dependencies: [
        {
          prerequisite: "plan-parser",
          dependent: "plan-tests",
          fulfilledBy: "parser-result",
        },
      ],
      nodes: [
        {
          id: "plan-parser",
          supersededBy: "parser-v2",
          expectedResult: { kind: "semantic_changes", requiresHandle: true },
          semanticScope: ["entity-parser"],
          claim: {
            id: "claim-parser",
            node: "plan-parser",
            branch: "parser-branch",
            owner: "worker-1",
            generation: 2,
          },
          assignment: {
            providerId: "provider-1",
            modelId: "model-a",
            thinkingLevel: "careful",
          },
        },
      ],
    });
    expect(session.branches[1]?.result).toEqual({
      id: "result-1",
      references: ["migration-1"],
      resultHandles: ["artifact-1"],
    });
    expect(await client.getBranch("session-1" as never, "child" as never)).toMatchObject({
      branchId: "child",
      status: "completed",
    });
    expect(await client.listBranchChildren("session-1" as never, "root" as never)).toMatchObject({
      items: [{ branchId: "child" }],
    });
    expect(await client.listBranchActivity("session-1" as never, "root" as never)).toMatchObject({
      items: [
        {
          id: "tool-call-1",
          kind: "tool",
          activityKind: "ToolInvocation",
          toolArguments: { query: "architecture", token: "[REDACTED]" },
        },
        {
          id: "tool-result-1",
          kind: "tool",
          activityKind: "ToolResult",
          parentId: "tool-call-1",
          handles: ["tool-result-1"],
        },
      ],
    });
    expect(await client.getBranchResult("session-1" as never, "result-1")).toEqual({
      branchId: "child",
      resultId: "result-1",
      handles: ["migration-1", "artifact-1"],
    });
  });
});
