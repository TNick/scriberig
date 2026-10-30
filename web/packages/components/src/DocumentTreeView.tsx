/** @file Renders semantic document sections, paragraphs, tables, and edit candidates. */
import { useEffect, useState, type ReactElement } from "react";
import type {
  DocumentBlockView as DocumentBlock,
  DocumentInspectResponse,
  DocumentParagraphEditRequest,
  DocumentParagraphEditResponse,
} from "@scriberig/protocol";
import type { DocumentPageController } from "@scriberig/model";
import { ProjectCandidateController } from "@scriberig/model";
import type { ProjectCandidateClient } from "@scriberig/model";
import { artifactCapabilityAllowsEdit } from "@scriberig/model";
import { DocumentParagraphEditor } from "./DocumentParagraphEditor";
import { DocumentTableCellEditor } from "./DocumentTableCellEditor";
import { ProjectCandidateReview } from "./ProjectCandidateReview";

/** Reads and renders one exact-snapshot semantic document tree.
 * @param props Page controller and client capability for paragraph candidates.
 * @returns Loading state, semantic document structure, or a safe read error.
 */
export function DocumentTreeView({
  controller,
  client,
  onSelectParagraph,
  onSelectTableCell,
  onCandidateApplied,
}: {
  /** Exact-project/snapshot controller. */
  controller: DocumentPageController;
  /** Shared client for candidate review, validation, and apply. */
  client: ProjectCandidateClient;
  /** Publishes a stable paragraph identity from the loaded snapshot. */
  onSelectParagraph?: (paragraph: string, snapshot: string) => void;
  /** Publishes a stable table-cell identity from the loaded snapshot. */
  onSelectTableCell?: (cell: string, snapshot: string) => void;
  /** Publishes the successor snapshot after candidate apply. */
  onCandidateApplied?: (snapshot: string) => void;
}): ReactElement {
  const [tree, setTree] = useState<DocumentInspectResponse | undefined>(controller.currentTree());
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(tree === undefined);
  const [candidateId, setCandidateId] = useState<string>();
  const [candidateSummary, setCandidateSummary] = useState<string>();
  const candidateController =
    candidateId === undefined ? undefined : new ProjectCandidateController(client, candidateId);
  const editClient = {
    proposeDocumentParagraphEdit: (
      request: DocumentParagraphEditRequest,
    ): Promise<DocumentParagraphEditResponse> =>
      controller.replaceParagraph(request.paragraph, request.replacement),
  };
  const cellEditClient = {
    proposeDocumentTableCellEdit: (
      request: import("@scriberig/protocol").DocumentTableCellEditRequest,
    ): Promise<DocumentParagraphEditResponse> =>
      controller.replaceTableCell(request.cell, request.replacement),
  };

  useEffect(() => {
    const abort = new AbortController();
    setLoading(true);
    setError(undefined);
    setCandidateId(undefined);
    setCandidateSummary(undefined);
    void controller
      .read(abort.signal)
      .then((result) => {
        if (!abort.signal.aborted) setTree(result);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      })
      .finally(() => {
        if (!abort.signal.aborted) setLoading(false);
      });
    return () => abort.abort();
  }, [controller]);

  return (
    <section aria-label="Semantic document">
      {loading ? <p role="status">Loading document…</p> : null}
      {error !== undefined ? <p role="alert">{error}</p> : null}
      {tree?.sections.map((section) => (
        <section key={section.id} aria-label={`Document section ${section.id}`}>
          <h2>Section</h2>
          {section.blocks.map((block) => (
            <DocumentBlockView
              key={block.kind === "paragraph" ? block.paragraph.id : block.table.id}
              block={block}
              tree={tree}
              editClient={editClient}
              cellEditClient={cellEditClient}
              onCandidateCreated={(candidate, summary) => {
                setCandidateId(candidate.candidateId);
                setCandidateSummary(summary);
              }}
              {...(onSelectParagraph === undefined ? {} : { onSelectParagraph })}
              {...(onSelectTableCell === undefined ? {} : { onSelectTableCell })}
            />
          ))}
        </section>
      ))}
      {candidateController ? (
        <ProjectCandidateReview
          key={candidateId}
          controller={candidateController}
          {...(candidateSummary === undefined ? {} : { summary: candidateSummary })}
          onApplied={(result) => onCandidateApplied?.(result.successor)}
        />
      ) : null}
    </section>
  );
}

/** Renders one paragraph or a nested semantic table block.
 * @param props Block, owning exact tree context, and candidate client.
 * @returns Read-only content with an edit candidate control when supported.
 */
function DocumentBlockView({
  block,
  tree,
  editClient,
  cellEditClient,
  onCandidateCreated,
  onSelectParagraph,
  onSelectTableCell,
  allowParagraphEdit = true,
}: {
  /** Semantic block to render. */
  block: DocumentBlock;
  /** Exact project and snapshot context. */
  tree: DocumentInspectResponse;
  /** Shared candidate operation. */
  editClient: Parameters<typeof DocumentParagraphEditor>[0]["client"];
  /** Stable table-cell candidate operation. */
  cellEditClient: Parameters<typeof DocumentTableCellEditor>[0]["client"];
  /** Publishes a candidate to the shared review surface. */
  onCandidateCreated: (candidate: DocumentParagraphEditResponse, summary: string) => void;
  /** Optional stable paragraph selection callback. */
  onSelectParagraph?: (paragraph: string, snapshot: string) => void;
  /** Optional stable table-cell selection callback. */
  onSelectTableCell?: (cell: string, snapshot: string) => void;
  /** Whether nested paragraph controls are available for this block. */
  allowParagraphEdit?: boolean;
}): ReactElement {
  if (block.kind === "paragraph") {
    const text = block.paragraph.runs.map((run) => run.text).join("");
    const editable =
      artifactCapabilityAllowsEdit(tree.artifactCapability) &&
      block.paragraph.runs.length === 1 &&
      block.paragraph.runs[0]?.hyperlink === undefined;
    return (
      <article aria-label="Document paragraph" data-paragraph-id={block.paragraph.id}>
        {onSelectParagraph ? (
          <button
            type="button"
            onClick={() => onSelectParagraph(block.paragraph.id, tree.snapshot)}
          >
            Select paragraph {block.paragraph.id}
          </button>
        ) : null}
        <p>{text}</p>
        {editable ? (
          allowParagraphEdit ? (
            <DocumentParagraphEditor
              client={editClient}
              project={tree.project}
              snapshot={tree.snapshot}
              paragraph={block.paragraph.id}
              text={text}
              onCandidateCreated={onCandidateCreated}
            />
          ) : null
        ) : (
          <p role="status">This paragraph structure is currently read-only.</p>
        )}
      </article>
    );
  }
  return (
    <table aria-label="Document table" data-table-id={block.table.id}>
      <tbody>
        {block.table.rows.map((row) => (
          <tr key={row.id} data-row-id={row.id}>
            {row.cells.map((cell) => {
              const paragraph =
                cell.blocks.length === 1 && cell.blocks[0]?.kind === "paragraph"
                  ? cell.blocks[0].paragraph
                  : undefined;
              const cellText = paragraph?.runs.map((run) => run.text).join("") ?? "";
              const cellEditable =
                paragraph !== undefined &&
                artifactCapabilityAllowsEdit(tree.artifactCapability) &&
                paragraph.runs.length === 1 &&
                paragraph.runs[0]?.hyperlink === undefined;
              return (
                <td key={cell.id} data-cell-id={cell.id}>
                  {onSelectTableCell ? (
                    <button type="button" onClick={() => onSelectTableCell(cell.id, tree.snapshot)}>
                      Select table cell {cell.id}
                    </button>
                  ) : null}
                  {cell.blocks.map((nested) => (
                    <DocumentBlockView
                      key={nested.kind === "paragraph" ? nested.paragraph.id : nested.table.id}
                      block={nested}
                      tree={tree}
                      editClient={editClient}
                      cellEditClient={cellEditClient}
                      onCandidateCreated={onCandidateCreated}
                      allowParagraphEdit={!cellEditable}
                      {...(onSelectParagraph === undefined ? {} : { onSelectParagraph })}
                      {...(onSelectTableCell === undefined ? {} : { onSelectTableCell })}
                    />
                  ))}
                  {cellEditable ? (
                    <DocumentTableCellEditor
                      client={cellEditClient}
                      project={tree.project}
                      snapshot={tree.snapshot}
                      cell={cell.id}
                      text={cellText}
                      onCandidateCreated={onCandidateCreated}
                    />
                  ) : null}
                </td>
              );
            })}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
