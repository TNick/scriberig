/** @file Provides styling-neutral trust and provenance components. */

import type { ReactElement } from "react";
import type { TrustInspectorState } from "@scriberig/model";
import type { CapabilityDenial, TrustDetails, TrustSummary } from "@scriberig/protocol";

/**
 * Renders compact source and validation state without a trust score.
 * @returns A compact trust summary element.
 */
export function TrustSummaryView({ summary }: { summary: TrustSummary }): ReactElement {
  return (
    <section aria-label="Trust summary">
      <strong>{summary.source}</strong>
      <span> validation: {summary.validation}</span>
      {summary.acceptedVia ? <span> accepted via: {summary.acceptedVia}</span> : null}
    </section>
  );
}

/**
 * Renders exact provenance handles.
 * @returns A provenance element.
 */
export function ProvenanceInspector({ details }: { details: TrustDetails }): ReactElement {
  return (
    <section aria-label="Provenance">
      {details.provenance.map((input) => (
        <div key={`${input.entityId}:${input.snapshotId}:${input.relation}`}>
          {input.relation}: {input.entityId}
        </div>
      ))}
    </section>
  );
}

/**
 * Renders validation diagnostics without collapsing states.
 * @returns A validation diagnostics list.
 */
export function ValidationList({ details }: { details: TrustDetails }): ReactElement {
  return (
    <ul aria-label="Validation diagnostics">
      {details.diagnostics.map((diagnostic) => (
        <li key={diagnostic}>{diagnostic}</li>
      ))}
    </ul>
  );
}

/**
 * Renders policy and approval handles.
 * @returns A policy handle element.
 */
export function PolicyEvaluationView({ details }: { details: TrustDetails }): ReactElement {
  return <p>Policy: {details.policyHandle ?? "not available"}</p>;
}

/**
 * Renders a denial without restricted payload content.
 * @returns A safe denial element.
 */
export function CapabilityDeniedNotice({ denial }: { denial: CapabilityDenial }): ReactElement {
  return (
    <section role="alert">
      <strong>Access denied</strong>
      <p>{denial.reason}</p>
      {denial.alternatives.map((alternative) => (
        <span key={alternative}>Alternative: {alternative}</span>
      ))}
    </section>
  );
}

/**
 * Renders a conflict handle when one exists.
 * @returns A conflict summary element.
 */
export function ConflictSummary({ details }: { details: TrustDetails }): ReactElement {
  return <p>Conflict: {details.conflictHandle ?? "none"}</p>;
}

/**
 * Renders the headless inspector state for an integrated editor surface.
 * @returns The appropriate trust state element.
 */
export function TrustInspectorPanel({ state }: { state: TrustInspectorState }): ReactElement {
  if (state.status === "idle" || state.status === "loading") {
    return (
      <p role="status">
        {state.status === "loading" ? "Loading trust…" : "Select an object to inspect trust."}
      </p>
    );
  }
  if (state.status === "denied") return <CapabilityDeniedNotice denial={state.error.denial} />;
  if (state.status === "error") return <p role="alert">Trust inspection failed.</p>;
  return (
    <>
      <TrustSummaryView summary={state.details.summary} />
      <ProvenanceInspector details={state.details} />
      <ValidationList details={state.details} />
      <PolicyEvaluationView details={state.details} />
      <ConflictSummary details={state.details} />
    </>
  );
}
