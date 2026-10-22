/** @file Verifies shared session, plan, preference, and operation projections. */
import { describe, expect, it } from "vitest";
import { normalizeSession } from "../src/session-views.js";
import { normalizePresentationPreferencesResponse } from "../src/presentation-views.js";
import { normalizeOperation } from "../src/response-normalizers.js";
import sharedFixture from "../../../../tests/fixtures/agent_session_plan_conformance.json";

describe("shared session projections", () => {
  it("preserves recursive session, plan, result, and preference semantics", () => {
    const fixture = sharedFixture as {
      session: unknown;
      presentation: unknown;
      operation: unknown;
    };
    const session = normalizeSession(fixture.session);
    const preferences = normalizePresentationPreferencesResponse(fixture.presentation);
    const operation = normalizeOperation(fixture.operation);
    const worker = session.branches.find((branch) => branch.id === "branch-worker-1");
    const plan = session.plans[0];

    expect(worker?.result).toEqual({
      id: "branch-result-1",
      references: ["migration-parser-1"],
      resultHandles: ["validation-1"],
    });
    expect(
      session.branches[0]?.activity?.find((item) => item.id === "activity-candidate-1"),
    ).toMatchObject({ kind: "SemanticOperation", durableRef: "migration-parser-1" });
    expect(
      session.branches[0]?.activity?.find((item) => item.id === "activity-validation-1"),
    ).toMatchObject({ kind: "Validation", durableRef: "validation-1" });
    expect(plan).toMatchObject({
      revision: 2,
      status: "Running",
      dependencies: [
        {
          prerequisite: "plan-node-parser",
          dependent: "plan-node-tests",
          fulfilledBy: "branch-result-1",
        },
      ],
    });
    expect(plan?.nodes[0]).toMatchObject({
      id: "plan-node-parser",
      assignment: {
        providerId: "provider-1",
        modelId: "model-a",
        thinkingLevel: "careful",
      },
    });
    expect(preferences.preferences).toEqual({
      reasoning: "shown",
      toolInvocation: "detailed",
      toolOutput: "summary",
      prompt: "hidden",
    });
    expect(operation).toMatchObject({
      operationId: "operation-agent-1",
      status: "completed",
      result: {
        branchResultId: "branch-result-1",
        producedChanges: ["entity-parser"],
        validationResults: ["validation passed"],
        unresolvedQuestions: [],
        candidateChanges: [
          {
            migrationId: "migration-parser-1",
            entityId: "entity-parser",
            candidateSnapshotId: "snapshot-parser-candidate-1",
            validationRefs: ["validation-1"],
          },
        ],
      },
    });
  });
});
