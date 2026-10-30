/** @file Verifies document navigation and edits retain exact semantic targets. */
import { describe, expect, it, vi } from "vitest";
import paragraphEdit from "../../../../tests/fixtures/document-paragraph-edit.json";
import { DocumentPageController } from "../src";

describe("DocumentPageController", () => {
  it("proposes the shared DOCX paragraph fixture edit", async () => {
    const calls: unknown[] = [];
    const client = {
      inspectDocument: async () => ({
        project: paragraphEdit.project,
        snapshot: paragraphEdit.snapshot,
        document: "document-root",
        sections: [
          {
            id: "section-1",
            blocks: [
              {
                kind: "paragraph" as const,
                paragraph: {
                  id: paragraphEdit.paragraph,
                  runs: [{ id: "run-1", text: paragraphEdit.paragraphText }],
                },
              },
            ],
          },
        ],
      }),
      proposeDocumentParagraphEdit: async (request: unknown) => {
        calls.push(request);
        return {
          project: paragraphEdit.project,
          candidateId: paragraphEdit.candidateId,
          baseSnapshot: paragraphEdit.snapshot,
          path: paragraphEdit.path,
          validation: { status: "pending" as const },
        };
      },
    };
    const controller = new DocumentPageController(
      client as never,
      paragraphEdit.project,
      paragraphEdit.snapshot,
      "document-root",
    );
    await controller.read();
    const candidate = await controller.replaceParagraph(
      paragraphEdit.paragraph,
      paragraphEdit.replacement,
    );

    expect(candidate.candidateId).toBe(paragraphEdit.candidateId);
    expect(calls).toEqual([
      {
        project: paragraphEdit.project,
        snapshot: paragraphEdit.snapshot,
        paragraph: paragraphEdit.paragraph,
        replacement: paragraphEdit.replacement,
      },
    ]);
  });

  it("reads one exact tree and proposes edits only for paragraph IDs in that tree", async () => {
    const calls: Array<[string, unknown]> = [];
    const client = {
      inspectDocument: vi.fn(async (request: unknown) => {
        calls.push(["inspect", request]);
        return {
          project: "project-1",
          snapshot: "snapshot-1",
          document: "document-1",
          sections: [
            {
              id: "section-1",
              blocks: [
                {
                  kind: "table",
                  table: {
                    id: "table-1",
                    rows: [
                      {
                        id: "row-1",
                        cells: [
                          {
                            id: "cell-1",
                            blocks: [
                              {
                                kind: "paragraph",
                                paragraph: {
                                  id: "paragraph-1",
                                  runs: [{ id: "run", text: "Old" }],
                                },
                              },
                            ],
                          },
                        ],
                      },
                    ],
                  },
                },
              ],
            },
          ],
        };
      }),
      proposeDocumentParagraphEdit: vi.fn(async (request: unknown) => {
        calls.push(["edit", request]);
        return {
          project: "project-1",
          candidateId: "candidate-1",
          baseSnapshot: "snapshot-1",
          path: "report.docx",
          validation: { status: "pending" as const },
        };
      }),
      proposeDocumentTableCellEdit: vi.fn(async (request: unknown) => {
        calls.push(["cell-edit", request]);
        return {
          project: "project-1",
          candidateId: "candidate-cell-1",
          baseSnapshot: "snapshot-1",
          path: "report.docx",
          validation: { status: "pending" as const },
        };
      }),
      proposeDocumentParagraphInsert: vi.fn(async (request: unknown) => {
        calls.push(["insert", request]);
        return {
          project: "project-1",
          candidateId: "candidate-2",
          baseSnapshot: "snapshot-1",
          path: "report.docx",
          validation: { status: "pending" as const },
        };
      }),
    };
    const controller = new DocumentPageController(
      client as never,
      "project-1",
      "snapshot-1",
      "document-1",
    );
    await controller.read();
    const candidate = await controller.replaceParagraph("paragraph-1", "New");
    expect(candidate.candidateId).toBe("candidate-1");
    const cellCandidate = await controller.replaceTableCell("cell-1", "New cell");
    expect(cellCandidate.candidateId).toBe("candidate-cell-1");
    const inserted = await controller.appendParagraph("section-1", "Appended");
    expect(inserted.candidateId).toBe("candidate-2");
    await expect(controller.replaceParagraph("foreign-paragraph", "No")).rejects.toThrow(
      RangeError,
    );
    await expect(controller.replaceTableCell("foreign-cell", "No")).rejects.toThrow(RangeError);
    await expect(controller.appendParagraph("foreign-section", "No")).rejects.toThrow(RangeError);
    expect(calls).toEqual([
      ["inspect", { project: "project-1", snapshot: "snapshot-1", document: "document-1" }],
      [
        "edit",
        {
          project: "project-1",
          snapshot: "snapshot-1",
          paragraph: "paragraph-1",
          replacement: "New",
        },
      ],
      [
        "cell-edit",
        {
          project: "project-1",
          snapshot: "snapshot-1",
          cell: "cell-1",
          replacement: "New cell",
        },
      ],
      [
        "insert",
        {
          project: "project-1",
          snapshot: "snapshot-1",
          section: "section-1",
          text: "Appended",
        },
      ],
    ]);
  });
});
