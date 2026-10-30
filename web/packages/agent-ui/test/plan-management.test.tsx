/** @file Verifies plan creation and lifecycle controls use shared views. */

import type { BranchDescriptor, SessionPlan } from "@scriberig/protocol";
import { ScriberigProvider } from "@scriberig/react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { PlanCreateForm, PlanLifecycleActions, PlanReplanForm } from "../src/plan-management-forms";

describe("plan management forms", () => {
  it("offers plan creation at an available branch head", () => {
    const branches = [
      {
        branchId: "branch-1" as never,
        name: "main",
        status: "active",
        childCount: 0,
        activityCount: 0,
      },
    ] as BranchDescriptor[];
    const markup = renderToStaticMarkup(
      <ScriberigProvider
        value={{ client: {} as never, model: {} as never, selection: {} as never, commands: [] }}
      >
        <PlanCreateForm
          sessionId={"session-1" as never}
          branches={branches}
          onPlanUpdated={async () => undefined}
        />
      </ScriberigProvider>,
    );

    expect(markup).toContain('aria-label="Create plan"');
    expect(markup).toContain('<option value="branch-1">main</option>');
    expect(markup).toContain("Create plan");
  });

  it("offers only lifecycle actions valid for the current plan status", () => {
    const plan = {
      id: "plan-1",
      revision: 4,
      goal: "Ship feature",
      status: "Planned",
      rootBranch: "branch-1" as never,
      nodes: [],
      dependencies: [],
      rootNodes: [],
      statusCounts: { planned: 1, ready: 0, running: 0, completed: 0, failed: 0, abandoned: 0 },
      gates: [],
      evidence: [],
      summaries: [],
      history: [],
    } as SessionPlan;
    const markup = renderToStaticMarkup(
      <ScriberigProvider
        value={{ client: {} as never, model: {} as never, selection: {} as never, commands: [] }}
      >
        <PlanLifecycleActions
          sessionId={"session-1" as never}
          plan={plan}
          onPlanUpdated={async () => undefined}
        />
      </ScriberigProvider>,
    );

    expect(markup).toContain(">Start</button>");
    expect(markup).toContain(">Stop</button>");
    expect(markup).not.toContain(">Complete</button>");
  });

  it("offers a replacement form for an unsuperseded failed node", () => {
    const plan = {
      id: "plan-1",
      revision: 5,
      goal: "Ship feature",
      status: "Failed",
      rootBranch: "branch-1" as never,
      dependencies: [],
      rootNodes: ["node-1", "node-0"],
      statusCounts: { planned: 0, ready: 0, running: 0, completed: 1, failed: 1, abandoned: 0 },
      gates: [],
      evidence: [],
      summaries: [],
      history: [],
      nodes: [
        {
          id: "node-1",
          parent: "",
          title: "Broken step",
          purpose: "Implement a feature",
          status: "Failed",
          containsIgnored: false,
          containsAbandoned: false,
          semanticScope: [],
          dependencyBlockers: [],
          entryGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          completionGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 0,
          canRetry: false,
        },
        {
          id: "node-0",
          parent: "",
          title: "Completed step",
          purpose: "Keep the accepted result",
          status: "Completed",
          containsIgnored: false,
          containsAbandoned: false,
          semanticScope: [],
          dependencyBlockers: [],
          entryGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          completionGates: { pending: 0, satisfied: 0, failed: 0, waived: 0 },
          evidenceCount: 1,
          canRetry: false,
        },
      ],
    } as SessionPlan;
    const markup = renderToStaticMarkup(
      <ScriberigProvider
        value={{ client: {} as never, model: {} as never, selection: {} as never, commands: [] }}
      >
        <PlanReplanForm
          sessionId={"session-1" as never}
          plan={plan}
          node={plan.nodes[0]!}
          onPlanUpdated={async () => undefined}
        />
      </ScriberigProvider>,
    );

    expect(markup).toContain('aria-label="Replace failed node Broken step"');
    expect(markup).toContain("Replace failed work");
  });
});
