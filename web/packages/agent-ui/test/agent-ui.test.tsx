/** @file Verifies reusable agent UI renders semantic, provider-neutral controls. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentComposer } from "../src/agent-panel";
import { BranchActivityView } from "../src/branch-activity";
import { AgentOperationPanel } from "../src/operation-panel";
import { SessionBranchTree } from "../src/session-panel";

describe("agent UI", () => {
  it("renders a selection-aware composer without provider fields", () => {
    const html = renderToStaticMarkup(
      <AgentComposer
        value="summarize"
        onChange={() => undefined}
        onSubmit={() => undefined}
        selection={{ entityId: "ent-1" as never }}
      />,
    );
    expect(html).toContain("Selection: ent-1");
    expect(html).toContain("Start agent");
    expect(html).not.toContain("api-key");
  });

  it("renders result-first branches and transient output separately", () => {
    const branchHtml = renderToStaticMarkup(
      <SessionBranchTree
        branches={[
          {
            branchId: "branch-1" as never,
            name: "child",
            status: "completed",
            childCount: 0,
            activityCount: 2,
            result: { branchId: "branch-1" as never, resultId: "result-1", handles: [] },
          },
        ]}
      />,
    );
    const operationHtml = renderToStaticMarkup(
      <AgentOperationPanel
        operation={{ operationId: "op-1" as never, kind: "agent", status: "running" }}
        stream="partial"
      />,
    );
    expect(branchHtml).toContain("completed");
    expect(branchHtml).toContain("result-1");
    expect(operationHtml).toContain("partial");
  });

  it("exposes collapsed nested branches and bounded activity accessibly", () => {
    const html = renderToStaticMarkup(
      <>
        <SessionBranchTree
          branches={[
            {
              branchId: "root" as never,
              name: "root",
              status: "completed",
              childCount: 1,
              activityCount: 0,
            },
            {
              branchId: "child" as never,
              parentBranchId: "root" as never,
              name: "child",
              status: "completed",
              childCount: 0,
              activityCount: 1,
            },
          ]}
        />
        <BranchActivityView
          page={{
            branchId: "root" as never,
            items: [{ id: "turn-1", ordinal: 1, kind: "model", summary: "done", handles: [] }],
          }}
        />
      </>,
    );
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain("Branch activity");
  });

  it("renders semantic result handles as keyboard-operable navigation controls", () => {
    const html = renderToStaticMarkup(
      <SessionBranchTree
        branches={[
          {
            branchId: "branch-1" as never,
            name: "child",
            status: "completed",
            childCount: 0,
            activityCount: 0,
            result: { branchId: "branch-1" as never, resultId: "result-1", handles: ["entity-1"] },
          },
        ]}
        onNavigate={() => undefined}
      />,
    );
    expect(html).toContain("Open result result-1");
    expect(html).toContain("Open entity-1");
    expect(html.match(/<button/g)?.length).toBe(3);
  });

  it("exposes candidate changes through the existing review action", () => {
    const html = renderToStaticMarkup(
      <AgentOperationPanel
        operation={{
          operationId: "op-1" as never,
          kind: "agent",
          status: "completed",
          result: {
            sessionId: "session-1" as never,
            rootBranchId: "branch-1" as never,
            branchResultId: "result-1" as never,
            finalOutput: "done",
            handles: [],
            candidateChanges: [
              {
                migrationId: "migration-1" as never,
                validationRefs: [],
              },
            ],
          },
        }}
        onReviewCandidate={() => undefined}
      />,
    );
    expect(html).toContain("Review candidate migration-1");
  });
});
