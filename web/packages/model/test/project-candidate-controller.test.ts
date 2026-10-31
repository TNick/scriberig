/** @file Verifies candidate review stays bound to its durable identity and validation. */
import { describe, expect, it, vi } from "vitest";
import { ProjectCandidateController } from "../src";
import { ServerProtocolError } from "@scriberig/client";

const review = (validation: "pending" | "passed" | "failed") => ({
  candidate: "candidate-1",
  project: "project-1",
  base: "snapshot-1",
  entity: "entity-1",
  path: "budget.xlsx",
  pathTruncated: false,
  validation,
  diagnostics: [],
  effects: [],
  effectCount: 0,
  effectsOmitted: 0,
  sourceBytesIncluded: false as const,
  semanticChanges: [],
  semanticChangesOmitted: 0,
});

describe("ProjectCandidateController", () => {
  it("requires shared passing validation before applying the same candidate", async () => {
    const calls: string[] = [];
    const project = vi.fn(async <T>(operation: string): Promise<T> => {
      calls.push(operation);
      if (operation === "candidate-summary") return review("pending") as T;
      if (operation === "validate-candidate-summary") return review("passed") as T;
      return {
        candidate: "candidate-1",
        base: "snapshot-1",
        successor: "snapshot-2",
        path: "budget.xlsx",
        paths: ["budget.xlsx"],
        materialization: "applied",
        history: true,
      } as T;
    });
    const controller = new ProjectCandidateController({ project } as never, "candidate-1");
    await controller.read();
    await expect(controller.apply()).rejects.toThrow("must pass validation");
    expect(calls).toEqual(["candidate-summary"]);
    await controller.validate();
    const result = await controller.apply();
    expect(result.successor).toBe("snapshot-2");
    expect(calls).toEqual(["candidate-summary", "validate-candidate-summary", "apply"]);
  });

  it("rejects a review response for a different candidate", async () => {
    const project = vi.fn(async () => ({ ...review("pending"), candidate: "other" }));
    const controller = new ProjectCandidateController({ project } as never, "candidate-1");
    await expect(controller.read()).rejects.toThrow("identity does not match");
  });

  it("rejects apply receipts for a different reviewed base snapshot", async () => {
    const project = vi.fn(async <T>(operation: string): Promise<T> => {
      if (operation === "candidate-summary") return review("pending") as T;
      if (operation === "validate-candidate-summary") return review("passed") as T;
      return {
        candidate: "candidate-1",
        base: "stale-base",
        successor: "snapshot-2",
        path: "budget.xlsx",
        paths: ["budget.xlsx"],
        materialization: "applied",
        history: true,
      } as T;
    });
    const controller = new ProjectCandidateController({ project } as never, "candidate-1");
    await controller.read();
    await controller.validate();
    await expect(controller.apply()).rejects.toThrow("does not match the reviewed candidate");
  });

  it("preserves durable path recovery receipts when materialization fails", async () => {
    const project = vi.fn(async <T>(operation: string): Promise<T> => {
      if (operation === "candidate-summary") return review("pending") as T;
      if (operation === "validate-candidate-summary") return review("passed") as T;
      throw new ServerProtocolError({
        code: "operation_failure",
        message: "one source path could not be replaced",
        details: {
          candidate: "candidate-1",
          materialization: "partial",
          diagnostic: "recovery is required",
          paths: [
            { path: "a.docx", state: "applied" },
            { path: "b.docx", state: "failed", diagnostic: "permission denied" },
          ],
        },
      });
    });
    const controller = new ProjectCandidateController({ project } as never, "candidate-1");
    await controller.read();
    await controller.validate();
    await expect(controller.apply()).rejects.toMatchObject({
      name: "ProjectCandidateApplyFailureError",
      failure: {
        candidate: "candidate-1",
        materialization: "partial",
        paths: [
          { path: "a.docx", state: "applied" },
          { path: "b.docx", state: "failed", diagnostic: "permission denied" },
        ],
      },
    });
  });
});
