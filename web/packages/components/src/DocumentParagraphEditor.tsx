/** @file Provides a reviewable semantic paragraph replacement control. */
import { useState, type FormEvent, type ReactElement } from "react";
import type {
  DocumentParagraphEditRequest,
  DocumentParagraphEditResponse,
} from "@scriberig/protocol";
import { documentParagraphChangeSummary } from "./document-change-summary";

/** Client capability required by the paragraph editor. */
interface DocumentEditClient {
  /** Creates an exact-base paragraph candidate. */
  /**
   * @param request Exact project, snapshot, paragraph, and replacement text.
   * @returns Reviewable candidate metadata.
   */
  proposeDocumentParagraphEdit(
    request: DocumentParagraphEditRequest,
  ): Promise<DocumentParagraphEditResponse>;
}

/** Creates a candidate from one exact-snapshot paragraph identity.
 * @param props Semantic target, current text, and shared client.
 * @returns Editor form and candidate status.
 */
export function DocumentParagraphEditor({
  client,
  project,
  snapshot,
  paragraph,
  text,
  onCandidateCreated,
}: {
  /** Shared semantic client. */
  client: DocumentEditClient;
  /** Stable owning project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic paragraph identity. */
  paragraph: string;
  /** Current semantic paragraph text. */
  text: string;
  /** Publishes the created candidate to the shared review surface. */
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
      const result = await client.proposeDocumentParagraphEdit({
        project,
        snapshot,
        paragraph,
        replacement,
      });
      setCandidate(result);
      onCandidateCreated?.(result, documentParagraphChangeSummary(paragraph, text, replacement));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  }

  return (
    <form aria-label="Edit document paragraph" onSubmit={(event) => void submit(event)}>
      <label>
        Replacement paragraph text
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
