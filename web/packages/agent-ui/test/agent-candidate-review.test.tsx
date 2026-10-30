/** @file Verifies rich agent candidate handles open the shared review action. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { BranchActivityView } from "../src/branch-activity";
import { SessionBranchTree } from "../src/session-panel";

describe("rich candidate review handles", () => {
  it("offers durable candidate handles through the shared review action", () => {
    const branchTree = renderToStaticMarkup(
      <SessionBranchTree
        branches={[
          {
            branchId: "branch-candidate" as never,
            name: "rich edit",
            status: "completed",
            childCount: 0,
            activityCount: 0,
            result: {
              branchId: "branch-candidate" as never,
              resultId: "result-candidate",
              handles: ["candidate:pc_docx_1", "entity:ent_paragraph"],
            },
          },
        ]}
        onNavigate={() => undefined}
        onReviewCandidate={() => undefined}
      />,
    );
    const activity = renderToStaticMarkup(
      <BranchActivityView
        page={{
          branchId: "branch-candidate" as never,
          items: [
            {
              id: "tool-result-candidate",
              ordinal: 1,
              kind: "tool",
              summary: "DOCX paragraph candidate created",
              handles: ["candidate:pc_docx_1"],
              activityKind: "ToolResult",
            },
          ],
        }}
        onNavigate={() => undefined}
        onReviewCandidate={() => undefined}
      />,
    );

    expect(branchTree).toContain("Review candidate pc_docx_1");
    expect(branchTree).not.toContain("Open candidate:pc_docx_1");
    expect(branchTree).toContain("Open entity:ent_paragraph");
    expect(activity).toContain("Review candidate pc_docx_1");
    expect(activity).not.toContain("Open candidate:pc_docx_1");
  });
});
