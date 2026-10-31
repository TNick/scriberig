/** @file Loads and presents the selected exact-snapshot semantic CAD drawing. */
import { useMemo, type ReactElement } from "react";
import type { ProjectCandidateApplyResult } from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";
import { CadPageController } from "@scriberig/model";
import { CadDrawingView, type CadProjectedEntitySelection } from "@scriberig/components";

/** Renders the selected drawing through shared semantic read and candidate operations.
 * @param props Shared client and exact project, snapshot, and drawing identities.
 * @returns Semantic CAD view or loading guidance.
 */
export function CadView({
  client,
  project,
  snapshot,
  drawing,
  onSelectEntity,
  onCandidateApplied,
}: {
  /** Shared semantic client. */ client: ScriberigClient;
  /** Active project identity. */ project: string;
  /** Snapshot recorded on the current selection. */ snapshot: string;
  /** Selected semantic CAD drawing identity. */ drawing: string;
  /** Publishes an exact semantic entity selected from the vector preview. */
  onSelectEntity: (selection: CadProjectedEntitySelection) => void;
  /** Publishes a successful shared candidate application receipt. */
  onCandidateApplied: (result: ProjectCandidateApplyResult) => void;
}): ReactElement {
  const controller = useMemo(
    () => new CadPageController(client, project, snapshot, drawing),
    [client, drawing, project, snapshot],
  );
  return (
    <CadDrawingView
      controller={controller}
      client={client}
      onSelectEntity={onSelectEntity}
      onCandidateApplied={onCandidateApplied}
    />
  );
}
