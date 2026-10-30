/** @file Verifies document semantic summaries retain exact targets and bound text. */
import { describe, expect, it } from "vitest";
import { documentParagraphChangeSummary } from "../src/document-change-summary";

describe("documentParagraphChangeSummary", () => {
  it("shows the paragraph handle and exact old-to-new text", () => {
    expect(documentParagraphChangeSummary("paragraph-1", "Before", "After")).toBe(
      "Paragraph paragraph-1 changes from “Before” to “After”.",
    );
  });

  it("bounds long previews without splitting Unicode code points", () => {
    const summary = documentParagraphChangeSummary("paragraph-2", "界".repeat(121), "replacement");
    expect(summary).toContain("Paragraph paragraph-2 changes from “" + "界".repeat(120) + "…”");
    expect(Array.from(summary).length).toBeLessThan(300);
  });
});
