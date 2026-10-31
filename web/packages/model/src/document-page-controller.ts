/** @file Coordinates exact-snapshot semantic document reads and paragraph edits. */
import type {
  DocumentInspectResponse,
  DocumentParagraphEditResponse,
  DocumentParagraphInsertResponse,
} from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Controls one document view and its candidate-only paragraph operation.
 *
 * Public members: `read`, `replaceParagraph`, `appendParagraph`, and `currentTree`.
 * Private members: client, project, exact snapshot, document ID, and latest tree.
 */
export class DocumentPageController {
  private tree?: DocumentInspectResponse;

  /** Creates a document controller bound to exact semantic identities.
   * @param client Shared client for document read and edit operations.
   * @param project Stable owning project identity.
   * @param snapshot Exact immutable snapshot identity.
   * @param document Stable semantic document root identity.
   */
  constructor(
    private readonly client: Pick<
      ScriberigClient,
      | "inspectDocument"
      | "proposeDocumentParagraphEdit"
      | "proposeDocumentTableCellEdit"
      | "proposeDocumentParagraphInsert"
    >,
    private readonly project: string,
    private readonly snapshot: string,
    private readonly document: string,
  ) {}

  /** Reads the complete bounded semantic document tree.
   * @param signal Optional request cancellation signal.
   * @returns Ordered document sections and nested semantic blocks.
   */
  async read(signal?: AbortSignal): Promise<DocumentInspectResponse> {
    const tree = await this.client.inspectDocument(
      { project: this.project, snapshot: this.snapshot, document: this.document },
      signal,
    );
    this.tree = tree;
    return tree;
  }

  /** Proposes a replacement for one stable paragraph ID in the loaded document.
   * @param paragraph Stable semantic paragraph identity.
   * @param replacement Authored replacement text.
   * @param signal Optional request cancellation signal.
   * @returns Shared reviewable candidate handle.
   * @throws When the paragraph is outside the loaded tree or the server rejects it.
   */
  replaceParagraph(
    paragraph: string,
    replacement: string,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphEditResponse> {
    if (!this.containsParagraph(paragraph)) {
      return Promise.reject(new RangeError("paragraph is not present in the loaded document"));
    }
    return this.client.proposeDocumentParagraphEdit(
      { project: this.project, snapshot: this.snapshot, paragraph, replacement },
      signal,
    );
  }

  /** Proposes a replacement for one simple stable table cell in the loaded tree.
   * @param cell Stable semantic table-cell identity.
   * @param replacement Replacement authored text.
   * @param signal Optional request cancellation signal.
   * @returns Shared reviewable candidate handle.
   * @throws When the cell is outside the loaded tree or the server rejects it.
   */
  replaceTableCell(
    cell: string,
    replacement: string,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphEditResponse> {
    if (!this.containsTableCell(cell)) {
      return Promise.reject(new RangeError("table cell is not present in the loaded document"));
    }
    return this.client.proposeDocumentTableCellEdit(
      { project: this.project, snapshot: this.snapshot, cell, replacement },
      signal,
    );
  }

  /** Proposes a paragraph append to one section in the loaded exact document.
   * @param section Stable section identity in the loaded document tree.
   * @param text Authored text for the appended paragraph.
   * @param signal Optional request cancellation signal.
   * @returns Shared reviewable candidate handle.
   * @throws When the section is outside the loaded tree or the server rejects it.
   */
  appendParagraph(
    section: string,
    text: string,
    signal?: AbortSignal,
  ): Promise<DocumentParagraphInsertResponse> {
    if (!this.tree?.sections.some((item) => item.id === section)) {
      return Promise.reject(new RangeError("section is not present in the loaded document"));
    }
    return this.client.proposeDocumentParagraphInsert(
      { project: this.project, snapshot: this.snapshot, section, text },
      signal,
    );
  }

  /** Reads the latest successfully loaded semantic tree.
   * @returns Loaded tree or undefined before the first successful read.
   */
  currentTree(): DocumentInspectResponse | undefined {
    return this.tree;
  }

  /** Verifies a paragraph identity occurs in this exact document tree.
   * @param paragraph Candidate paragraph identity.
   * @returns True if the identity is present in a section or nested table cell.
   */
  private containsParagraph(paragraph: string): boolean {
    return (
      this.tree?.sections.some((section) => this.hasParagraph(section.blocks, paragraph)) ?? false
    );
  }

  /** Verifies a table-cell identity occurs in this exact document tree.
   * @param cell Candidate table-cell identity.
   * @returns True if the identity is present in a table or nested cell.
   */
  private containsTableCell(cell: string): boolean {
    return this.tree?.sections.some((section) => this.hasTableCell(section.blocks, cell)) ?? false;
  }

  /** Searches semantic block IDs recursively for a table cell.
   * @param blocks Ordered blocks to inspect.
   * @param cell Target table-cell identity.
   * @returns True when the cell occurs in these blocks or nested tables.
   */
  private hasTableCell(
    blocks: DocumentInspectResponse["sections"][number]["blocks"],
    cell: string,
  ): boolean {
    return blocks.some(
      (block) =>
        block.kind === "table" &&
        block.table.rows.some((row) =>
          row.cells.some((value) => value.id === cell || this.hasTableCell(value.blocks, cell)),
        ),
    );
  }

  /** Searches semantic block IDs recursively through table cells.
   * @param blocks Ordered blocks to inspect.
   * @param paragraph Target semantic paragraph identity.
   * @returns True when the paragraph occurs in these blocks or nested cells.
   */
  private hasParagraph(
    blocks: DocumentInspectResponse["sections"][number]["blocks"],
    paragraph: string,
  ): boolean {
    return blocks.some((block) =>
      block.kind === "paragraph"
        ? block.paragraph.id === paragraph
        : block.table.rows.some((row) =>
            row.cells.some((cell) => this.hasParagraph(cell.blocks, paragraph)),
          ),
    );
  }
}
