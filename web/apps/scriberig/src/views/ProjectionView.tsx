/** @file Provides the public projection host surface. */
import { MuiObjectInspector, MuiProjectionHost } from "@scriberig/components-mui";
import type { ProjectionHandler } from "@scriberig/components";
import { useObject, useSelection, useScriberig, useSnapshot } from "@scriberig/react";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import { Button } from "@mui/material";
import type { AppRuntime, ProjectCandidate } from "../runtime";

interface SourceProjection {
  path: string;
  text: string;
  name?: string;
  snapshot: string;
  editable: boolean;
}

/** Renders the built-in projection from one exact semantic selection.
 * @returns The rendered default projection state.
 */
function DefaultProjection(): ReactElement {
  const { selection } = useSelection();
  const request = useMemo(
    () =>
      selection
        ? {
            entityId: selection.entityId,
            snapshot: { kind: "exact" as const, snapshotId: selection.snapshotId },
            detail: "deep" as const,
          }
        : undefined,
    [selection],
  );
  const object = useObject(request);
  const snapshot = useSnapshot(selection?.snapshotId);

  if (!selection) return <p role="status">Select a semantic object to render.</p>;
  if (object.status === "loading") return <p role="status">Rendering projection…</p>;
  if (object.status === "error") {
    return <p role="alert">Unable to render the selected semantic object.</p>;
  }
  if (object.status !== "loaded") return <p role="status">Projection is unavailable.</p>;
  return (
    <MuiObjectInspector
      value={{
        summary: object.value.summary,
        ...(object.value.text === undefined ? {} : { description: object.value.text }),
        state:
          snapshot.kind === "candidate"
            ? "historical"
            : snapshot.kind === "current"
              ? "current"
              : snapshot.kind,
      }}
    />
  );
}
/** Hosts registered projection handlers without private fallbacks. */
/** @returns The rendered ProjectionView view or runtime value. */
export function ProjectionView({
  handlers = [],
}: {
  handlers?: readonly ProjectionHandler[];
}): ReactElement {
  const { selection } = useSelection();
  const runtime = useScriberig() as unknown as AppRuntime;
  const [projection, setProjection] = useState<SourceProjection>();
  const [editedText, setEditedText] = useState("");
  const [candidate, setCandidate] = useState<ProjectCandidate>();
  const [message, setMessage] = useState<string>();
  useEffect(() => {
    if (!selection || !runtime.activeWorkspace) {
      setProjection(undefined);
      return;
    }
    void runtime.client
      .project<SourceProjection>("projection", {
        project: runtime.activeWorkspace.project.id,
        entity: selection.entityId,
      })
      .then((value) => {
        setProjection(value);
        setEditedText(value.text);
        setCandidate(runtime.agentCandidate);
        setMessage(undefined);
      })
      .catch(() => setProjection(undefined));
  }, [runtime, selection]);
  return (
    <section aria-label="Projection">
      <h1>Projection</h1>
      {projection ? (
        <section aria-label="Rust source projection">
          <p>{projection.path}</p>
          <p>snapshot: {projection.snapshot}</p>
          <textarea
            aria-label="Source projection"
            value={editedText}
            onChange={(event) => setEditedText(event.target.value)}
            readOnly={!projection.editable}
            rows={20}
            cols={100}
          />
          <p>{projection.editable ? "Editable" : "Read-only"}</p>
          {projection.editable ? (
            <Button
              variant="contained"
              onClick={() => {
                if (!runtime.activeWorkspace || !selection) return;
                void runtime.client
                  .project<ProjectCandidate>("propose", {
                    project: runtime.activeWorkspace.project.id,
                    entity: selection.entityId,
                    text: editedText,
                  })
                  .then((value) => {
                    runtime.setAgentCandidate(value);
                    setCandidate(value);
                  })
                  .catch((error: unknown) => setMessage(String(error)));
              }}
            >
              Review change
            </Button>
          ) : null}
        </section>
      ) : null}
      {candidate ? (
        <section aria-label="Migration review">
          <h2>Migration Review</h2>
          <p>Base snapshot: {candidate.base}</p>
          <p>Candidate: {candidate.candidate}</p>
          <p>Path: {candidate.path}</p>
          <p>add</p>
          <pre>{`old: ${candidate.old}\nnew: ${candidate.new}`}</pre>
          <p>Validation: {candidate.validation}</p>
          <Button
            variant="contained"
            disabled={!candidate.applyEnabled}
            onClick={() => {
              void runtime.client
                .project<{ successor: string }>("apply", { candidate: candidate.candidate })
                .then(async (value) => {
                  setMessage(`Applied; successor snapshot ${value.successor}`);
                  if (runtime.activeWorkspace) {
                    await runtime.activateProject(runtime.activeWorkspace.project.id);
                  }
                  runtime.setAgentCandidate(undefined);
                })
                .catch((error: unknown) => setMessage(String(error)));
            }}
          >
            Apply migration
          </Button>
        </section>
      ) : null}
      {message ? <p role="status">{message}</p> : null}
      {handlers.length > 0 ? (
        <MuiProjectionHost
          projectionId="default"
          handlers={handlers}
          readonly
          {...(selection ? { selection: JSON.stringify(selection) } : {})}
        />
      ) : (
        <DefaultProjection />
      )}
    </section>
  );
}
