/** @file Provides a reviewable semantic table-cell replacement control. */
import { useState, type FormEvent, type ReactElement } from "react";
import type {
  DocumentParagraphEditResponse,
  DocumentTableCellEditRequest,
} from "@scriberig/protocol";
import { documentTableCellChangeSummary } from "./document-change-summary";

/** Client capability required by the table-cell editor. */
interface DocumentTableCellEditClient {
  /** Creates an exact-base table-cell candidate.
   * @param request Exact project, snapshot, cell, and replacement text.
   * @returns Reviewable candidate metadata.
   */
  proposeDocumentTableCellEdit(
    request: DocumentTableCellEditRequest,
  ): Promise<DocumentParagraphEditResponse>;
}

/** Creates a candidate from one exact-snapshot table-cell identity.
 * @param props Semantic target, current text, and shared client.
 * @returns Editor form and candidate status.
 */
export function DocumentTableCellEditor({
  client,
  project,
  snapshot,
  cell,
  text,
  onCandidateCreated,
}: {
  /** Shared semantic client. */
  client: DocumentTableCellEditClient;
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic table-cell identity. */
  cell: string;
  /** Current semantic cell text. */
  text: string;
  /** Publishes the candidate to the shared review surface. */
  onCandidateCreated?: (candidate: DocumentParagraphEditResponse, summary: string) => void;
}): ReactElement {
  const [replacement, setReplacement] = useState(text);
  const [candidate, setCandidate] = useState<DocumentParagraphEditResponse>();
  const [error, setError] = useState<string>();
  const [saving, setSaving] = useState(false);

  /** Submits the replacement as a candidate for normal review and apply.
   * @param event Form submission event.
   * @returns No value.
   */
  async function submit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setSaving(true);
    setError(undefined);
    setCandidate(undefined);
    try {
      const result = await client.proposeDocumentTableCellEdit({
        project,
        snapshot,
        cell,
        replacement,
      });
      setCandidate(result);
      onCandidateCreated?.(result, documentTableCellChangeSummary(cell, text, replacement));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  }

  return (
    <form aria-label="Edit document table cell" onSubmit={(event) => void submit(event)}>
      <label>
        Replacement table cell text
        <textarea value={replacement} onChange={(event) => setReplacement(event.target.value)} />
      </label>
      <button type="submit" disabled={saving || replacement === text}>
        {saving ? "Creating candidate…" : "Create edit candidate"}
      </button>
      {error !== undefined ? <p role="alert">{error}</p> : null}
      {candidate !== undefined ? (
        <p role="status">
          Candidate {candidate.candidateId} for {candidate.path} is {candidate.validation.status}.
        </p>
      ) : null}
    </form>
  );
}
