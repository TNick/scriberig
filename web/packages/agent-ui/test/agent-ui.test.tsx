/** @file Verifies reusable agent UI renders semantic, provider-neutral controls. */

import sensitiveActivityFixture from "../../../../tests/fixtures/agent_console_sensitive_activity.json";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { BranchActivityPage } from "@scriberig/protocol";
import { AgentComposer } from "../src/agent-panel";
import { AgentModelControls, AgentPresentationControls } from "../src/model-controls";
import { BranchActivityView } from "../src/branch-activity";
import { AgentOperationPanel } from "../src/operation-panel";
import { SessionBranchTree } from "../src/session-panel";

describe("agent UI", () => {
  it("renders the shared schema-redacted activity fixture without its secret marker", () => {
    const secretMarker = "FAKE_PROVIDER_SECRET_73B2";
    const fixture = sensitiveActivityFixture as unknown as {
      activityPage: BranchActivityPage;
    };
    const detailed = renderToStaticMarkup(
      <BranchActivityView
        page={fixture.activityPage}
        presentation={{
          reasoning: "shown",
          toolInvocation: "detailed",
          toolOutput: "summary",
          prompt: "hidden",
        }}
      />,
    );
    const simple = renderToStaticMarkup(
      <BranchActivityView
        page={fixture.activityPage}
        presentation={{
          reasoning: "hidden",
          toolInvocation: "simple",
          toolOutput: "summary",
          prompt: "hidden",
        }}
      />,
    );

    expect(detailed).toContain("[REDACTED]");
    expect(detailed).not.toContain(secretMarker);
    expect(detailed).toContain("Found one result");
    expect(simple).toContain("Find documents");
    expect(simple).not.toContain(secretMarker);
    for (const item of fixture.activityPage.items) {
      expect(simple).toContain(`data-activity-id="${item.id}"`);
      expect(detailed).toContain(`data-activity-id="${item.id}"`);
    }
  });

  it("renders independent presentation choices without execution controls", () => {
    const html = renderToStaticMarkup(
      <AgentPresentationControls
        value={{
          reasoning: "hidden",
          toolInvocation: "simple",
          toolOutput: "summary",
          prompt: "hidden",
        }}
        onChange={() => undefined}
      />,
    );
    expect(html).toContain('aria-label="Reasoning visibility"');
    expect(html).toContain('aria-label="Tool invocation detail"');
    expect(html).toContain('aria-label="Tool output visibility"');
    expect(html).toContain('aria-label="Prompt visibility"');
  });

  it("renders only the selected model's advertised thinking levels", () => {
    const html = renderToStaticMarkup(
      <AgentModelControls
        library={{
          models: [
            {
              id: "model-a",
              displayName: "Model A",
              providerId: "local",
              modelId: "wire-a",
              capabilities: { streaming: true, toolCalls: true, structuredOutput: false },
              thinkingLevels: [{ id: "brief", label: "Brief", order: 1 }],
            },
            {
              id: "model-b",
              displayName: "Model B",
              providerId: "local",
              modelId: "wire-b",
              capabilities: { streaming: true, toolCalls: true, structuredOutput: false },
              thinkingLevels: [{ id: "deep", label: "Deep", order: 1 }],
            },
          ],
          profiles: [
            {
              id: "profile-a",
              displayName: "Profile A",
              model: "model-a",
              connection: "local",
              enabled: true,
            },
            {
              id: "profile-b",
              displayName: "Profile B",
              model: "model-b",
              connection: "local",
              enabled: true,
            },
          ],
        }}
        selection={{
          revision: 2,
          providerId: "local",
          modelId: "wire-a",
          thinkingLevel: "brief",
        }}
        onModelChange={() => undefined}
        onThinkingChange={() => undefined}
      />,
    );

    expect(html).toContain("Model A");
    expect(html).toContain("Brief");
    expect(html).toContain('aria-label="Agent model"');
    expect(html).toContain('aria-label="Agent thinking level"');
    expect(html).not.toContain(">Deep</option>");
  });

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

  it("keeps the composer editable while a separate running operation can cancel", () => {
    const html = renderToStaticMarkup(
      <>
        <AgentComposer value="follow-up" onChange={() => undefined} onSubmit={() => undefined} />
        <AgentOperationPanel
          operation={{
            operationId: "op-running" as never,
            kind: "agent",
            status: "running",
            cancellable: true,
          }}
          onCancel={() => undefined}
        />
      </>,
    );

    expect(html).toContain('aria-label="Agent composer"');
    expect(html).toContain("follow-up");
    expect(html).toContain(">Start agent</button>");
    expect(html.indexOf("</form>")).toBeLessThan(html.indexOf(">Cancel</button>"));
    expect(html).not.toMatch(/<textarea[^>]*disabled/);
    expect(html).toContain(">Cancel</button>");
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
    expect(operationHtml).toContain('aria-live="off"');
    expect(operationHtml).not.toContain("<output");
  });

  it("shows captured model-request sections only when prompt visibility is enabled", () => {
    const page = {
      branchId: "root" as never,
      sessionId: "session-1" as never,
      items: [
        {
          id: "invocation-1",
          ordinal: 0,
          kind: "model" as const,
          activityKind: "ModelInvocation" as const,
          summary: "Model request: model-a",
          handles: ["invocation-1"],
        },
      ],
    };
    const preferences = {
      reasoning: "hidden" as const,
      toolInvocation: "simple" as const,
      toolOutput: "summary" as const,
      prompt: "hidden" as const,
    };
    const hidden = renderToStaticMarkup(
      <BranchActivityView page={page} presentation={preferences} />,
    );
    const shown = renderToStaticMarkup(
      <BranchActivityView page={page} presentation={{ ...preferences, prompt: "shown" }} />,
    );

    expect(hidden).not.toContain("Model request</summary>");
    expect(shown).toContain("Model request</summary>");
    expect(shown).toContain("Captured request unavailable");
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

  it("renders transient runtime activity as a separate live tail", () => {
    const html = renderToStaticMarkup(
      <BranchActivityView
        page={{ branchId: "root" as never, items: [] }}
        presentation={{
          reasoning: "shown",
          toolInvocation: "simple",
          toolOutput: "summary",
          prompt: "hidden",
        }}
        transientItems={[
          {
            sequence: 7,
            operationId: "op-1",
            branchId: "root",
            activityKind: "ToolInvocation",
            state: "Running",
            summary: "Tool: search",
          },
        ]}
      />,
    );
    expect(html).toContain('aria-label="Live activity"');
    expect(html).toContain("Tool: search");
    expect(html).toContain('data-activity-state="Running"');

    const hidden = renderToStaticMarkup(
      <BranchActivityView
        page={{ branchId: "root" as never, items: [] }}
        presentation={{
          reasoning: "hidden",
          toolInvocation: "simple",
          toolOutput: "summary",
          prompt: "hidden",
        }}
        transientItems={[
          {
            sequence: 7,
            operationId: "op-1",
            branchId: "root",
            activityKind: "ReasoningSummary",
            state: "Completed",
            summary: "safe summary",
          },
        ]}
      />,
    );
    expect(hidden).not.toContain("Live activity");
    expect(hidden).not.toContain("safe summary");
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
            producedChanges: [],
            validationResults: [],
            unresolvedQuestions: [],
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
