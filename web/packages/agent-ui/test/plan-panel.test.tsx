/** @file Verifies semantic plan action controls. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { SessionPlan } from "@scriberig/protocol";
import { SessionPlanPanel } from "../src/plan-panel";

describe("session plan panel", () => {
  it("renders retry, gate, waiver, and evidence actions", () => {
    const plan: SessionPlan = {
      id: "plan-1",
      revision: 4,
      goal: "Ship feature",
      status: "Running",
      rootBranch: "branch-1" as never,
      nodes: [
        {
          id: "node-1",
          parent: "branch-1",
          title: "Implement",
          purpose: "Make the change",
          status: "Failed",
          semanticScope: [],
          containsIgnored: false,
          containsAbandoned: false,
          dependencyBlockers: [],
          entryGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          completionGates: { pending: 1, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 0,
          canRetry: true,
        },
      ],
      dependencies: [],
      rootNodes: ["node-1"],
      statusCounts: {
        planned: 0,
        ready: 0,
        running: 0,
        completed: 0,
        failed: 1,
        abandoned: 0,
      },
      gates: [
        {
          id: "gate-1",
          owner: "node-1",
          phase: "Completion",
          requirement: { EvidencePresent: { kind: "BranchResult" } },
        },
      ],
      evidence: [],
      summaries: [],
      history: [],
    };
    const html = renderToStaticMarkup(
      <SessionPlanPanel
        plans={[plan]}
        branches={[
          {
            branchId: "branch-2" as never,
            name: "completed worker",
            status: "completed",
            childCount: 0,
            activityCount: 1,
            result: {
              branchId: "branch-2" as never,
              resultId: "result-2",
              summary: "Validated change",
              handles: [],
            },
          },
        ]}
        onMutation={async () => undefined}
      />,
    );

    expect(html).toContain("Retry failed node");
    expect(html).toContain("Require branch-result evidence");
    expect(html).toContain("Waive gate");
    expect(html).toContain("Attach result from completed worker");
  });
});
