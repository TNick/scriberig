/** @file Verifies the shared candidate review shows bounded effects and safe actions. */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ProjectCandidateFailureStatus, ProjectCandidateReview } from "../src";

describe("ProjectCandidateReview", () => {
  it("renders aggregate recovery state and each durable path receipt", () => {
    const html = renderToStaticMarkup(
      <ProjectCandidateFailureStatus
        failure={{
          code: "filesystem_write_failed",
          candidate: "candidate-1",
          materialization: "partial",
          diagnostic: "recovery is required",
          paths: [
            { path: "a.docx", state: "applied" },
            { path: "b.docx", state: "failed", diagnostic: "permission denied" },
          ],
          pathCount: 40,
          pathsOmitted: 38,
        }}
      />,
    );
    expect(html).toContain("Materialization recovery status");
    expect(html).toContain("materialization is partial");
    expect(html).toContain("a.docx: applied");
    expect(html).toContain("b.docx: failed");
    expect(html).toContain("permission denied");
    expect(html).toContain("Showing 2 of 40 path receipts; 38 are omitted.");
  });

  it("shows the exact candidate, bounded changes, and shared validation controls", () => {
    const controller = {
      current: () => ({
        candidate: "candidate-1",
        project: "project-1",
        base: "snapshot-1",
        entity: "cell-1",
        semanticOperation: "spreadsheet.set_cell",
        semanticTarget: "cell-1",
        semanticChanges: [
          {
            target: "cell-1",
            property: "cell.value",
            before: '{"Number":"10"}',
            beforeTruncated: false,
            after: '{"Number":"12"}',
            afterTruncated: false,
          },
          {
            target: "cell-2",
            property: "table_cell.paragraph_text",
            before: '"parcel 124"',
            beforeTruncated: false,
            after: '"parcel 125"',
            afterTruncated: false,
          },
        ],
        semanticChangesOmitted: 0,
        path: "budget.xlsx",
        pathTruncated: false,
        validation: "passed" as const,
        diagnostics: ["XLSX package validation passed"],
        effects: [
          {
            path: "budget.xlsx",
            pathTruncated: false,
            kind: "update" as const,
            oldByteLength: 100,
            newByteLength: 104,
          },
        ],
        effectCount: 1,
        effectsOmitted: 0,
        sourceBytesIncluded: false as const,
      }),
      read: async () => undefined,
    };
    const html = renderToStaticMarkup(
      <ProjectCandidateReview
        controller={controller as never}
        summary="Cell B3 changes from 10 to 12."
      />,
    );
    expect(html).toContain("Cell B3 changes from 10 to 12.");
    expect(html).toContain("candidate-1");
    expect(html).toContain("spreadsheet.set_cell");
    expect(html).toContain("cell-1");
    expect(html).toContain("cell.value");
    expect(html).toContain("Number");
    expect(html).toContain("Number &quot;10&quot;");
    expect(html).toContain("Number &quot;12&quot;");
    expect(html).toContain("table_cell.paragraph_text");
    expect(html).toContain("&quot;parcel 124&quot;");
    expect(html).toContain("&quot;parcel 125&quot;");
    expect(html).toContain("budget.xlsx");
    expect(html).toContain("Validate candidate");
    expect(html).toContain("XLSX package validation passed");
    expect(html).toContain("Apply candidate");
    expect(html).toContain("Source bytes are kept out");
  });
});
