/** @file Verifies bounded candidate review responses reject raw source contents. */
import { describe, expect, it } from "vitest";
import { normalizeProjectCandidateApplyResult, normalizeProjectCandidateReview } from "../src";

describe("project candidate response normalization", () => {
  it("normalizes bounded source effects and apply receipts", () => {
    const review = normalizeProjectCandidateReview({
      candidate: "candidate-1",
      project: "project-1",
      base: "snapshot-1",
      entity: "entity-1",
      semantic_operation: "spreadsheet.set_cell",
      semantic_target: "cell-3",
      semantic_changes: [
        {
          target: "cell-3",
          property: "cell.value",
          before: '{"Number":"10"}',
          before_truncated: false,
          after: '{"Number":"12"}',
          after_truncated: false,
        },
        {
          target: "cell-4",
          property: "cell.value",
          before: '{"Text":"old"}',
          before_truncated: false,
          after: '{"Text":"new"}',
          after_truncated: false,
        },
      ],
      semantic_changes_omitted: 0,
      path: "budget.xlsx",
      path_truncated: false,
      validation: "passed",
      diagnostics: [],
      effects: [
        {
          path: "budget.xlsx",
          path_truncated: false,
          kind: "update",
          old_byte_length: 100,
          new_byte_length: 104,
        },
      ],
      effect_count: 1,
      effects_omitted: 0,
      source_bytes_included: false,
    });
    expect(review.effects[0]?.newByteLength).toBe(104);
    expect(review.semanticOperation).toBe("spreadsheet.set_cell");
    expect(review.semanticTarget).toBe("cell-3");
    expect(review.semanticChanges[0]?.before).toBe('{"Number":"10"}');
    expect(review.semanticChanges).toHaveLength(2);
    expect(review.sourceBytesIncluded).toBe(false);
    expect(
      normalizeProjectCandidateApplyResult({
        candidate: "candidate-1",
        base: "snapshot-1",
        successor: "snapshot-2",
        path: "budget.xlsx",
        paths: ["budget.xlsx"],
        materialization: "applied",
        history: true,
      }).successor,
    ).toBe("snapshot-2");
  });

  it("rejects review responses that contain source bytes", () => {
    expect(() =>
      normalizeProjectCandidateReview({
        candidate: "candidate-1",
        project: "project-1",
        base: "snapshot-1",
        entity: "entity-1",
        path: "budget.xlsx",
        path_truncated: false,
        validation: "passed",
        diagnostics: [],
        effects: [],
        effect_count: 0,
        effects_omitted: 0,
        source_bytes_included: true,
      }),
    ).toThrow("must not contain source bytes");
  });
});
