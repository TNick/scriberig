/** @file Coordinates bounded review, validation, and apply for project candidates. */
import type { ScriberigClient } from "@scriberig/client";
import {
  normalizeProjectCandidateApplyResult,
  normalizeProjectCandidateReview,
} from "@scriberig/client";
import type { ProjectCandidateApplyResult, ProjectCandidateReview } from "@scriberig/protocol";
import { projectCandidateApplyFailure } from "./project-candidate-failure";

/** Public client capability required to review durable project candidates. */
export type ProjectCandidateClient = Pick<ScriberigClient, "project">;

/** Drives the shared durable project-candidate review lifecycle.
 *
 * Public members: read, validate, apply, and current.
 * Private members: project client, candidate identity, and current validation.
 */
export class ProjectCandidateController {
  private review?: ProjectCandidateReview;

  /** Creates a controller bound to one durable candidate.
   * @param client Client exposing project workflow operations.
   * @param candidate Stable candidate identity returned by candidate creation.
   */
  constructor(
    private readonly client: ProjectCandidateClient,
    private readonly candidate: string,
  ) {}

  /** Reads bounded candidate state without source bytes.
   * @param signal Optional request cancellation signal.
   * @returns Candidate review summary.
   * @throws When the response refers to a different candidate.
   */
  async read(signal?: AbortSignal): Promise<ProjectCandidateReview> {
    const review = normalizeProjectCandidateReview(
      await this.client.project<unknown>(
        "candidate-summary",
        { candidate: this.candidate },
        signal,
      ),
    );
    this.assertCandidate(review.candidate);
    this.review = review;
    return review;
  }

  /** Validates the durable candidate in the shared isolated workspace.
   * @param signal Optional request cancellation signal.
   * @returns Updated candidate review summary and validation state.
   * @throws When the response refers to a different candidate.
   */
  async validate(signal?: AbortSignal): Promise<ProjectCandidateReview> {
    const review = normalizeProjectCandidateReview(
      await this.client.project<unknown>(
        "validate-candidate-summary",
        { candidate: this.candidate },
        signal,
      ),
    );
    this.assertCandidate(review.candidate);
    this.review = review;
    return review;
  }

  /** Applies only a candidate whose latest shared review passed validation.
   * @param signal Optional request cancellation signal.
   * @returns Exact apply receipt with the successor snapshot.
   * @throws When validation has not passed or apply returns another candidate.
   */
  async apply(signal?: AbortSignal): Promise<ProjectCandidateApplyResult> {
    if (this.review?.validation !== "passed") {
      throw new TypeError("candidate must pass validation before apply");
    }
    let result: ProjectCandidateApplyResult;
    try {
      result = normalizeProjectCandidateApplyResult(
        await this.client.project<unknown>("apply", { candidate: this.candidate }, signal),
      );
    } catch (error) {
      const failure = projectCandidateApplyFailure(error);
      if (failure?.failure.candidate === this.candidate) throw failure;
      throw error;
    }
    if (result.candidate !== this.candidate || result.base !== this.review.base) {
      throw new TypeError("candidate apply result does not match the reviewed candidate");
    }
    return result;
  }

  /** Reads the latest completed review state.
   * @returns Candidate review summary, if one has been loaded.
   */
  current(): ProjectCandidateReview | undefined {
    return this.review;
  }

  /** Rejects review data that does not match this controller's exact candidate.
   * @param candidate Candidate identity returned by the server.
   * @throws When the returned identity differs from this controller.
   */
  private assertCandidate(candidate: string): void {
    if (candidate !== this.candidate) {
      throw new TypeError("candidate review identity does not match the requested candidate");
    }
  }
}
