/** @file Verifies shared plan data is rendered by the agent plan panel. */

import type { SessionPlan } from "@scriberig/protocol";
import { ScriberigProvider } from "@scriberig/react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PlanPanel } from "../src/plan-panel";
import { PlanNodeAuthoringForm } from "../src/plan-node-authoring-form";

describe("plan panel", () => {
  it("renders shared status, blockers, and retry capability", () => {
    const plan = {
      id: "plan-1",
      revision: 3,
      snapshot: "snapshot-3",
      goal: "Ship feature",
      status: "Running",
      rootBranch: "branch-1" as never,
      dependencies: [],
      rootNodes: ["node-1"],
      statusCounts: { planned: 0, ready: 0, running: 0, completed: 0, failed: 1, abandoned: 0 },
      gates: [],
      evidence: [],
      summaries: [],
      history: [],
      nodes: [
        {
          id: "node-1",
          parent: "",
          title: "Implement",
          purpose: "Write code",
          status: "Failed",
          effectiveStatus: "Failed",
          containsIgnored: false,
          containsAbandoned: false,
          semanticScope: [],
          dependencyBlockers: ["node-0"],
          entryGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          completionGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 1,
          canRetry: true,
        },
      ],
    } as SessionPlan;
    const markup = renderToStaticMarkup(
      <ScriberigProvider
        value={{ client: {} as never, model: {} as never, selection: {} as never, commands: [] }}
      >
        <PlanPanel
          sessionId={"session-1" as never}
          plans={[plan]}
          onPlanUpdated={async () => undefined}
        />
      </ScriberigProvider>,
    );
    expect(markup).toContain("Ship feature (Running)");
    expect(markup).toContain("Implement: Failed / Failed");
    expect(markup).toContain("Blocked by node-0");
    expect(markup).toContain(">Retry</button>");
    expect(markup).toContain('aria-label="Add next step to Ship feature"');
  });

  it("renders summary and typed gate authoring forms for a selected node", () => {
    const plan = {
      id: "plan-1",
      revision: 3,
      goal: "Ship feature",
      status: "Running",
      rootBranch: "branch-1" as never,
      dependencies: [],
      rootNodes: ["node-1"],
      statusCounts: { planned: 0, ready: 1, running: 0, completed: 0, failed: 0, abandoned: 0 },
      gates: [],
      evidence: [],
      summaries: [],
      history: [],
      nodes: [
        {
          id: "node-1",
          parent: "",
          title: "Implement",
          purpose: "Write code",
          status: "Ready",
          containsIgnored: false,
          containsAbandoned: false,
          summaryPreview: "Current progress",
          semanticScope: [],
          dependencyBlockers: [],
          entryGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          completionGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 0,
          canRetry: false,
        },
      ],
    } as SessionPlan;
    const markup = renderToStaticMarkup(
      <ScriberigProvider
        value={{ client: {} as never, model: {} as never, selection: {} as never, commands: [] }}
      >
        <PlanNodeAuthoringForm
          sessionId={"session-1" as never}
          plan={plan}
          node={plan.nodes[0]!}
          onPlanUpdated={async () => undefined}
        />
      </ScriberigProvider>,
    );

    expect(markup).toContain('aria-label="Update summary for Implement"');
    expect(markup).toContain('aria-label="Add gate to Implement"');
    expect(markup).toContain("Save summary");
    expect(markup).toContain("Add gate");
  });
});
