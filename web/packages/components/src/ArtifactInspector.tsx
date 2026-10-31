/** @file Renders bounded, provider-neutral artifact metadata. */
import type { ReactElement } from "react";
import type { ArtifactSummary } from "@scriberig/protocol";

/** Maximum member descriptors rendered from one artifact summary. */
const MAX_VISIBLE_MEMBERS = 20;

/** Renders artifact identity, source evidence, fidelity, references, and bounded members.
 * @param props Artifact summary and optional styling class.
 * @returns Reusable semantic artifact inspector.
 */
export function ArtifactInspector({
  artifact,
  className,
  onInspectMembers,
}: {
  /** Shared artifact summary returned by the application client. */
  artifact: ArtifactSummary;
  /** Optional caller-owned styling class. */
  className?: string;
  /** Opens the paged member evidence for this artifact. */
  onInspectMembers?: (artifactId: string) => void;
}): ReactElement {
  const visibleMembers = artifact.members.slice(0, MAX_VISIBLE_MEMBERS);
  return (
    <section className={className} aria-label="Artifact inspector">
      <h2>{artifact.logicalKey}</h2>
      <p>
        {artifact.kind} · {artifact.adapter}
        {artifact.adapterVersion ? ` ${artifact.adapterVersion}` : ""} · {artifact.fidelity}
      </p>
      <dl>
        <div>
          <dt>Artifact</dt>
          <dd>{artifact.id}</dd>
        </div>
        <div>
          <dt>Source scan</dt>
          <dd>{artifact.scanId}</dd>
        </div>
        <div>
          <dt>Opaque preservation</dt>
          <dd>{artifact.opaquePreservation}</dd>
        </div>
        <div>
          <dt>Synchronization</dt>
          <dd>{artifact.synchronizationStatus}</dd>
        </div>
        <div>
          <dt>Materialization baseline</dt>
          <dd>{artifact.baselineDigest || "unavailable"}</dd>
        </div>
        {artifact.observedDigest ? (
          <div>
            <dt>Observed source digest</dt>
            <dd>{artifact.observedDigest}</dd>
          </div>
        ) : null}
      </dl>
      {artifact.diagnostic ? <p role="status">{artifact.diagnostic}</p> : null}
      <ul aria-label="Artifact members">
        {visibleMembers.map((member) => (
          <li key={member.key}>
            {member.key} · {member.projectPath ?? member.containerPart ?? "package member"} ·{" "}
            {member.digest}
          </li>
        ))}
      </ul>
      {artifact.memberCount > visibleMembers.length ? (
        <p role="status">
          Showing {visibleMembers.length} of {artifact.memberCount} members.
        </p>
      ) : null}
      {artifact.memberCount > artifact.members.length && onInspectMembers ? (
        <button type="button" onClick={() => onInspectMembers(artifact.id)}>
          Inspect all {artifact.memberCount} members
        </button>
      ) : null}
      {artifact.references.length > 0 ? (
        <ul aria-label="External references">
          {artifact.references.map((reference, index) => (
            <li key={`${reference.kind}:${reference.target}:${index}`}>
              {reference.kind}: {reference.target}
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
