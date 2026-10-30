/** @file Reviews, validates, and applies a durable project candidate. */
import { useEffect, useState, type ReactElement } from "react";
import type {
  ProjectCandidateApplyResult,
  ProjectCandidateReview as ProjectCandidateReviewData,
} from "@scriberig/protocol";
import type { ProjectCandidateController } from "@scriberig/model";
import { ProjectCandidateApplyFailureError } from "@scriberig/model";
import { ProjectCandidateFailureStatus } from "./ProjectCandidateFailureStatus";

/** Presents the shared source-candidate review and apply lifecycle.
 * @param props Controller, candidate context, and optional apply callback.
 * @returns Bounded candidate details and authorized lifecycle controls.
 */
export function ProjectCandidateReview({
  controller,
  summary,
  onApplied,
}: {
  /** Shared controller bound to the durable candidate. */
  controller: ProjectCandidateController;
  /** Optional domain-level edit summary from the creating editor. */
  summary?: string;
  /** Called with the exact materialization receipt after successful apply. */
  onApplied?: (result: ProjectCandidateApplyResult) => void;
}): ReactElement {
  const [review, setReview] = useState<ProjectCandidateReviewData | undefined>(() =>
    controller.current(),
  );
  const [applied, setApplied] = useState<ProjectCandidateApplyResult>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [applyFailure, setApplyFailure] = useState<ProjectCandidateApplyFailureError>();

  useEffect(() => {
    const request = new AbortController();
    void controller
      .read(request.signal)
      .then((result) => {
        if (!request.signal.aborted) setReview(result);
      })
      .catch((reason: unknown) => {
        if (!request.signal.aborted) setError(String(reason));
      });
    return () => request.abort();
  }, [controller]);

  /** Runs shared validation and displays its durable result.
   * @returns Completion after validation state is refreshed.
   */
  async function validate(): Promise<void> {
    setBusy(true);
    setError(undefined);
    setApplyFailure(undefined);
    try {
      setReview(await controller.validate());
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  /** Applies a candidate after the shared validation state passes.
   * @returns Completion after the apply receipt is displayed.
   */
  async function apply(): Promise<void> {
    setBusy(true);
    setError(undefined);
    setApplyFailure(undefined);
    try {
      const result = await controller.apply();
      setApplied(result);
      onApplied?.(result);
    } catch (reason) {
      if (reason instanceof ProjectCandidateApplyFailureError) {
        setApplyFailure(reason);
      } else {
        setError(String(reason));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-label="Project candidate review">
      <h2>Review candidate</h2>
      {summary ? <p>{summary}</p> : null}
      {review ? (
        <>
          <dl>
            <dt>Candidate</dt>
            <dd>{review.candidate}</dd>
            <dt>Base snapshot</dt>
            <dd>{review.base}</dd>
            {review.semanticOperation ? (
              <>
                <dt>Semantic operation</dt>
                <dd>{review.semanticOperation}</dd>
              </>
            ) : null}
            {review.semanticTarget ? (
              <>
                <dt>Semantic target</dt>
                <dd>{review.semanticTarget}</dd>
              </>
            ) : null}
            <dt>Validation</dt>
            <dd>{review.validation}</dd>
            <dt>Changed files</dt>
            <dd>{review.effectCount}</dd>
          </dl>
          {review.semanticChanges.length > 0 ? (
            <ul aria-label="Semantic before and after changes">
              {review.semanticChanges.map((change, index) => (
                <li key={`${change.target}-${change.property}-${index}`}>
                  {change.property}: {semanticValueLabel(change.before)}
                  {change.beforeTruncated ? "…" : ""} → {semanticValueLabel(change.after)}
                  {change.afterTruncated ? "…" : ""}
                </li>
              ))}
            </ul>
          ) : null}
          {review.semanticChangesOmitted > 0 ? (
            <p role="status">
              {review.semanticChangesOmitted} additional semantic changes are omitted.
            </p>
          ) : null}
          <ul aria-label="Candidate source effects">
            {review.effects.map((effect, index) => (
              <li key={effect.path + "-" + index}>
                {effect.path}
                {effect.pathTruncated ? "…" : ""} ({effect.kind}; {effect.oldByteLength} →{" "}
                {effect.newByteLength} bytes)
              </li>
            ))}
          </ul>
          {review.effectsOmitted > 0 ? (
            <p role="status">{review.effectsOmitted} additional source effects are omitted.</p>
          ) : null}
          {review.diagnostics.length > 0 ? (
            <ul aria-label="Candidate validation diagnostics">
              {review.diagnostics.map((diagnostic, index) => (
                <li key={index + "-" + diagnostic}>{diagnostic}</li>
              ))}
            </ul>
          ) : null}
          <button type="button" disabled={busy} onClick={() => void validate()}>
            Validate candidate
          </button>
          <button
            type="button"
            disabled={busy || applied !== undefined || review.validation !== "passed"}
            onClick={() => void apply()}
          >
            Apply candidate
          </button>
          <p role="note">Source bytes are kept out of this bounded review summary.</p>
        </>
      ) : (
        <p role="status">Loading candidate review…</p>
      )}
      {busy ? <p role="status">Updating candidate…</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {applyFailure ? <ProjectCandidateFailureStatus failure={applyFailure.failure} /> : null}
      {applied ? (
        <p role="status">
          Applied at snapshot {applied.successor}; {applied.paths.length} source paths were
          materialized.
        </p>
      ) : null}
    </section>
  );
}

/** Formats one JSON-encoded typed value as a compact domain-readable preview.
 * @param serialized Bounded JSON preview from the shared candidate review model.
 * @returns Typed value label or the original text when the preview is truncated.
 */
function semanticValueLabel(serialized: string): string {
  let value: unknown;
  try {
    value = JSON.parse(serialized);
  } catch {
    return serialized;
  }
  if (typeof value === "string") return JSON.stringify(value);
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return String(value);
  }
  const [kind, payload] = Object.entries(value)[0] ?? [];
  if (!kind) return JSON.stringify(value);
  if (
    kind === "Formula" &&
    payload !== null &&
    typeof payload === "object" &&
    "expression" in payload &&
    typeof payload.expression === "string"
  ) {
    return `Formula ${payload.expression}`;
  }
  return `${kind} ${JSON.stringify(payload)}`;
}
