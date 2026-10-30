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
          prerequisite: "ent_04040404040404040404040404040404",
          dependent: "ent_05050505050505050505050505050505",
          fulfilledBy: "ent_0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e",
        },
      ],
    });
    expect(plan?.nodes[0]).toMatchObject({
      id: "ent_04040404040404040404040404040404",
      status: "Completed",
      effectiveStatus: "Completed",
      evidenceCount: 1,
      assignment: {
        providerId: "provider-1",
        modelId: "model-a",
        thinkingLevel: "careful",
      },
    });
    expect(plan?.nodes[1]).toMatchObject({
      id: "ent_05050505050505050505050505050505",
      effectiveStatus: "Ready",
      dependencyBlockers: [],
      completionGates: { pending: 0, satisfied: 1, failed: 0, waived: 0 },
      evidenceCount: 1,
      summaryId: "ent_0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c",
    });
    expect(plan?.nodes[2]?.lastFailure).toMatchObject({
      kind: "Failure",
      reference: {
        entity: "ent_06060606060606060606060606060606",
        revision: "rev_0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f",
      },
      attributes: { category: "transient" },
    });
    expect(plan?.gates).toHaveLength(1);
    expect(plan?.evidence).toHaveLength(2);
    expect(plan?.summaries).toMatchObject([
      {
        id: "ent_0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c",
        node: "ent_05050505050505050505050505050505",
        text: "Parser checks passed.",
      },
    ]);
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
