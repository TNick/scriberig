/** @file Verifies document operations through the public client API. */
import { describe, expect, it } from "vitest";
import paragraphEdit from "../../../../tests/fixtures/document-paragraph-edit.json";
import { ScriberigClient } from "../src";

describe("ScriberigClient", () => {
  it("proposes a DOCX paragraph edit through the shared candidate operation", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: paragraphEdit.project,
          candidate_id: paragraphEdit.candidateId,
          base_snapshot: paragraphEdit.snapshot,
          path: paragraphEdit.path,
          validation: { status: paragraphEdit.validation },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: paragraphEdit.project,
      snapshot: paragraphEdit.snapshot,
      paragraph: paragraphEdit.paragraph,
      replacement: paragraphEdit.replacement,
    };
    const candidate = await new ScriberigClient(transport).proposeDocumentParagraphEdit(request);
    expect(candidate).toEqual({
      project: paragraphEdit.project,
      candidateId: paragraphEdit.candidateId,
      baseSnapshot: paragraphEdit.snapshot,
      path: paragraphEdit.path,
      validation: { status: paragraphEdit.validation },
    });
    expect(calls).toEqual([["document_paragraph_edit", request]]);
  });

  it("proposes a DOCX table-cell edit through the shared candidate operation", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          candidate_id: "candidate-cell-1",
          base_snapshot: "snapshot-1",
          path: "report.docx",
          validation: { status: "pending" },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      cell: "cell-1",
      replacement: "parcel 125",
    };
    const candidate = await new ScriberigClient(transport).proposeDocumentTableCellEdit(request);
    expect(candidate.candidateId).toBe("candidate-cell-1");
    expect(calls).toEqual([["document_table_cell_edit", request]]);
  });

  it("proposes a document paragraph insertion through the public protocol", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          candidate_id: "candidate-3",
          base_snapshot: "snapshot-1",
          path: "report.docx",
          validation: { status: "pending" },
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      section: "section-1",
      text: "Appended paragraph",
    };

    const candidate = await new ScriberigClient(transport).proposeDocumentParagraphInsert(request);

    expect(candidate.candidateId).toBe("candidate-3");
    expect(candidate.baseSnapshot).toBe("snapshot-1");
    expect(calls).toEqual([["document_paragraph_insert", request]]);
  });

  it("reads typed semantic document sections and normalizes nested table identities", async () => {
    const calls: Array<[string, unknown]> = [];
    const transport = {
      request: async <T>(operation: string, payload: unknown): Promise<T> => {
        calls.push([operation, payload]);
        return {
          project: "project-1",
          snapshot: "snapshot-1",
          document: "document-1",
          artifact_capability: "read_only_derived",
          styles: [{ key: "BodyText", name: "Body Text" }],
          sections: [
            {
              id: "section-1",
              blocks: [
                {
                  kind: "paragraph",
                  paragraph: {
                    id: "paragraph-1",
                    runs: [{ id: "run-1", text: "Read ", style: null, hyperlink: null }],
                    style: null,
                  },
                },
                {
                  kind: "table",
                  table: {
                    id: "table-1",
                    rows: [
                      {
                        id: "row-1",
                        cells: [{ id: "cell-1", blocks: [] }],
                      },
                    ],
                  },
                },
              ],
            },
          ],
        } as T;
      },
      subscribe: () => () => undefined,
    };
    const request = {
      project: "project-1",
      snapshot: "snapshot-1",
      document: "document-1",
    };
    const tree = await new ScriberigClient(transport).inspectDocument(request);
    expect(tree.artifactCapability).toBe("read_only_derived");
    expect(tree.styles).toEqual([{ key: "BodyText", name: "Body Text" }]);
    expect(tree.sections[0]?.blocks[0]).toEqual({
      kind: "paragraph",
      paragraph: {
        id: "paragraph-1",
        runs: [{ id: "run-1", text: "Read " }],
      },
    });
    expect(tree.sections[0]?.blocks[1]).toEqual({
      kind: "table",
      table: { id: "table-1", rows: [{ id: "row-1", cells: [{ id: "cell-1", blocks: [] }] }] },
    });
    expect(calls).toEqual([["document_inspect", request]]);
  });
});
