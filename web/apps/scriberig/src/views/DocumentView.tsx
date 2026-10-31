/** @file Loads and presents the selected exact-snapshot semantic document. */
import { useMemo, type ReactElement } from "react";
import type { ScriberigClient } from "@scriberig/client";
import { DocumentPageController } from "@scriberig/model";
import { DocumentTreeView } from "@scriberig/components";

/** Renders the semantic document selected in the shared workbench state.
 * @param props Client and exact project, snapshot, and document IDs.
 * @returns Semantic document outline/content or loading guidance.
 */
export function DocumentView({
  client,
  project,
  snapshot,
  document,
  onSelectParagraph,
  onSelectTableCell,
  onCandidateApplied,
}: {
  /** Shared semantic client. */
  client: ScriberigClient;
  /** Active project identity. */
  project: string;
  /** Snapshot recorded on the current semantic selection. */
  snapshot: string;
  /** Selected semantic document root identity. */
  document: string;
  /** Publishes a stable paragraph selection and its exact snapshot. */
  onSelectParagraph?: (paragraph: string, snapshot: string) => void;
  /** Publishes a stable table-cell selection and its exact snapshot. */
  onSelectTableCell?: (cell: string, snapshot: string) => void;
  /** Publishes the successor snapshot after shared candidate application. */
  onCandidateApplied?: (snapshot: string) => void;
}): ReactElement {
  const controller = useMemo(
    () => new DocumentPageController(client, project, snapshot, document),
    [client, document, project, snapshot],
  );
  return (
    <DocumentTreeView
      controller={controller}
      client={client}
      {...(onSelectParagraph === undefined ? {} : { onSelectParagraph })}
      {...(onSelectTableCell === undefined ? {} : { onSelectTableCell })}
      {...(onCandidateApplied === undefined ? {} : { onCandidateApplied })}
    />
  );
}
