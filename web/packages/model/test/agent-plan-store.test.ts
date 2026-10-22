/** @file Verifies authoritative agent plan updates. */

import { describe, expect, it } from "vitest";
import type { SessionPlan } from "@scriberig/protocol";
import {
  createAgentWorkspaceState,
  storeAgentPlan,
  storeAgentSession,
} from "../src/agent-workspace";

describe("agent plan store", () => {
  it("replaces a plan with the authoritative mutation result", () => {
    const plan: SessionPlan = {
      id: "plan-1",
      revision: 3,
      goal: "Ship feature",
      status: "Running",
      rootBranch: "branch-1" as never,
      nodes: [],
      dependencies: [],
      rootNodes: [],
      statusCounts: {
        planned: 0,
        ready: 0,
        running: 0,
        completed: 0,
        failed: 0,
        abandoned: 0,
      },
      gates: [],
      evidence: [],
      summaries: [],
      history: [],
    };
    let state = storeAgentSession(createAgentWorkspaceState(), {
      session: { id: "session-1" as never },
      execution: { revision: 0 },
      branches: [],
      plans: [{ ...plan, revision: 2 }],
    });

    state = storeAgentPlan(state, plan);

    expect(state.plans).toEqual([plan]);
  });
});
