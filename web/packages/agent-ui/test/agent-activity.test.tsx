/** @file Verifies causal activity rendering and presentation modes. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { BranchActivityView } from "../src/branch-activity";

describe("agent activity", () => {
  it("renders causal activity with 2px rails and local invocation detail", () => {
    const page = {
      branchId: "root" as never,
      items: [
        {
          id: "call-1",
          ordinal: 0,
          kind: "tool" as const,
          activityKind: "ToolInvocation" as const,
          summary: "Search semantic content",
          toolName: "search",
          toolArguments: { query: "architecture", token: "[REDACTED]" },
          handles: [],
        },
        {
          id: "result-1",
          ordinal: 1,
          kind: "tool" as const,
          activityKind: "ToolResult" as const,
          summary: "Tool completed",
          toolResultPreview: { summary: "safe full result", handles: ["entity-1"] },
          parentId: "call-1",
          handles: ["result-1"],
        },
        {
          id: "nested-result-1",
          ordinal: 2,
          kind: "tool" as const,
          activityKind: "ToolResult" as const,
          summary: "Nested result",
          parentId: "result-1",
          handles: [],
        },
      ],
    };
    const originalItems = structuredClone(page.items);
    let toolResultReads = 0;
    const onLoadToolResult = async () => {
      toolResultReads += 1;
      return { result: "result-1", detail: { summary: "safe full result" }, truncated: false };
    };
    const simple = renderToStaticMarkup(
      <BranchActivityView
        page={page}
        onLoadToolResult={onLoadToolResult}
        presentation={{
          reasoning: "hidden",
          toolInvocation: "simple",
          toolOutput: "hidden",
          prompt: "hidden",
        }}
      />,
    );
    const detailed = renderToStaticMarkup(
      <BranchActivityView
        page={page}
        onLoadToolResult={onLoadToolResult}
        presentation={{
          reasoning: "shown",
          toolInvocation: "detailed",
          toolOutput: "full",
          prompt: "shown",
        }}
      />,
    );
    const summarized = renderToStaticMarkup(
      <BranchActivityView
        page={page}
        onLoadToolResult={onLoadToolResult}
        presentation={{
          reasoning: "hidden",
          toolInvocation: "simple",
          toolOutput: "summary",
          prompt: "hidden",
        }}
      />,
    );

    expect(simple).toContain("&gt; Search semantic content");
    expect(simple).toContain("--query");
    expect(simple).toContain('data-activity-id="call-1"');
    expect(simple).toContain("Tool result hidden");
    expect(simple).not.toContain("Tool completed");
    expect(simple).toContain('data-activity-id="result-1"');
    expect(simple.match(/aria-hidden="true" style="width:2px/g)?.length).toBe(3);
    expect(simple).not.toContain("border-inline-start");
    expect(detailed).toContain("Tool invocation");
    expect(detailed).toContain('data-activity-id="call-1"');
    expect(detailed).toContain("query");
    expect(detailed).toContain("[REDACTED]");
    expect(simple).not.toContain("SECRET_MARKER_DO_NOT_RENDER");
    expect(detailed).not.toContain("SECRET_MARKER_DO_NOT_RENDER");
    expect(summarized).toContain("Tool completed");
    expect(summarized).toContain('data-activity-id="result-1"');
    expect(detailed).toContain("safe full result");
    expect(detailed).toContain("Expand full result");
    expect(detailed).toContain('data-activity-id="result-1"');
    expect(page.items).toEqual(originalItems);
    expect(toolResultReads).toBe(0);
  });
});
