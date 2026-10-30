/** @file Connects the reusable style controls to shared resolution and preference operations. */

import { MuiProjectionStyleControls } from "@scriberig/components-mui";
import { useProjectionStyleController } from "@scriberig/react";
import type {
  ProjectionStyleResolveRequest,
  ProjectionStyleResolveResponse,
  ProjectionStyleSchemaResponse,
} from "@scriberig/protocol";
import { useEffect, useState, type ReactElement } from "react";

/** Resolves the selected projection's style and writes only the user's preference scope.
 * @param props Exact projection kind and named shared scopes.
 * @returns Reusable schema, provenance, fidelity, and personal preference controls.
 */
export function ProjectionStylePanel({
  projectionKind,
  projectId,
  workspaceId,
  viewId,
}: {
  /** Domain adapter projection kind from the project service. */
  projectionKind: string;
  /** Exact project identity used by shared style resolution. */
  projectId: string;
  /** Named workspace scope used by the resolver. */
  workspaceId: string;
  /** Stable view identity used by shared view override resolution. */
  viewId: string;
}): ReactElement {
  const controller = useProjectionStyleController();
  const [schema, setSchema] = useState<ProjectionStyleSchemaResponse>();
  const [supported, setSupported] = useState<boolean>();
  const [resolution, setResolution] = useState<ProjectionStyleResolveResponse>();
  const [preference, setPreference] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const request: ProjectionStyleResolveRequest = {
    projectionKind,
    projectId,
    workspaceId,
    purpose: "interactiveView",
    viewId,
  };

  useEffect(() => {
    const abort = new AbortController();
    setSchema(undefined);
    setSupported(undefined);
    setResolution(undefined);
    setPreference("");
    setError(undefined);
    void controller
      .schema(projectionKind, abort.signal)
      .then(async (resolvedSchema) => {
        if (abort.signal.aborted) return;
        if (!resolvedSchema) {
          setSupported(false);
          return;
        }
        const resolvedStyle = await controller.resolve(request, abort.signal);
        if (abort.signal.aborted) return;
        setSupported(true);
        setSchema(resolvedSchema);
        setResolution(resolvedStyle);
        setPreference(userPreferenceText(resolvedStyle));
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      });
    return () => abort.abort();
  }, [controller, projectionKind, projectId, workspaceId, viewId]);

  /** Persists or clears the personal preference through the shared controller.
   * @param style Canonical style bytes to save, or undefined to clear the preference.
   * @returns Resolves after the preference and effective style are refreshed.
   */
  async function updatePreference(style: number[] | undefined): Promise<void> {
    setBusy(true);
    setError(undefined);
    try {
      const current = resolution?.applied.find((source) => source.kind === "user-preference");
      const saved = await controller.set({
        projectionKind,
        scope: { kind: "user" },
        ...(current?.generation === undefined ? {} : { expectedRevision: current.generation }),
        ...(style === undefined ? {} : { style }),
      });
      setPreference(styleText(saved.style));
      setResolution(await controller.resolve(request));
    } catch (reason: unknown) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  if (error) return <p role="alert">Projection style is unavailable: {error}</p>;
  if (supported === false) return <></>;
  if (supported !== true || !schema || !resolution) {
    return <p role="status">Resolving projection style…</p>;
  }
  return (
    <>
      <MuiProjectionStyleControls
        schema={schema}
        resolution={resolution}
        preference={preference}
        busy={busy}
        onPreferenceChange={setPreference}
        onSave={() => void updatePreference(Array.from(new TextEncoder().encode(preference)))}
        onClear={() => void updatePreference(undefined)}
      />
      {busy ? <p role="status">Updating personal style…</p> : null}
    </>
  );
}

/** Reads one exact user style input as UTF-8 for the generic text control.
 * @param resolution Shared resolution containing the applied source chain.
 * @returns User preference text, or an empty string when none is configured.
 */
function userPreferenceText(resolution: ProjectionStyleResolveResponse): string {
  const source = resolution.applied.find((item) => item.kind === "user-preference");
  return source === undefined ? "" : (styleText(source.value) ?? "");
}

/** Decodes adapter-owned style bytes only when they are valid UTF-8.
 * @param bytes Optional bytes returned by the shared style contract.
 * @returns UTF-8 input, or an empty string when the style is absent/non-text.
 */
function styleText(bytes: number[] | undefined): string {
  if (bytes === undefined) return "";
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array(bytes));
  } catch {
    return "";
  }
}
