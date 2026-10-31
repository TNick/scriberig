/** @file Verifies accessible, styling-neutral component behavior. */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  ArtifactInspector,
  CadDrawingView,
  DocumentParagraphEditor,
  DocumentTableCellEditor,
  DocumentTreeView,
  MigrationPreview,
  ObjectInspector,
  OperationView,
  ProjectionHost,
  SpreadsheetCellTable,
  SpreadsheetCellEditor,
  SpreadsheetPageView,
  SessionTree,
} from "../src";

describe("neutral components", () => {
  it("renders a typed semantic spreadsheet cell editor", () => {
    const html = renderToStaticMarkup(
      <SpreadsheetCellEditor
        controller={{} as never}
        cell={{
          id: "cell-3-2",
          row: 3,
          column: 2,
          value: {
            kind: "formula",
            value: { expression: "=A3*2", cachedValue: { kind: "number", value: "20" } },
          },
        }}
        artifactCapability="lossless_editable"
      />,
    );
    expect(html).toContain("Semantic cell: cell-3-2");
    expect(html).toContain("Formula expression");
    expect(html).toContain("Cached result");
    expect(html).toContain("20");
    expect(html).toContain("=A3*2");
    expect(html).toContain("Create edit candidate");
  });

  it("disables spreadsheet editing and shows the artifact diagnostic when read-only", () => {
    const html = renderToStaticMarkup(
      <SpreadsheetCellEditor
        controller={{} as never}
        cell={{ id: "cell-1", row: 1, column: 1, value: { kind: "text", value: "Parcel" } }}
        artifactCapability="read_only_derived"
        artifactDiagnostic="Workbook has unsupported formulas."
      />,
    );
    expect(html).toContain("Workbook has unsupported formulas.");
    expect(html).not.toContain("Create edit candidate");
    expect(html).not.toContain("<input");
  });

  it("renders an exact-target semantic document paragraph candidate form", () => {
    const html = renderToStaticMarkup(
      <DocumentParagraphEditor
        client={{
          proposeDocumentParagraphEdit: async () => ({
            project: "project-1",
            candidateId: "candidate-1",
            baseSnapshot: "snapshot-1",
            path: "report.docx",
            validation: { status: "pending" },
          }),
        }}
        project="project-1"
        snapshot="snapshot-1"
        paragraph="paragraph-1"
        text="Existing site area"
      />,
    );
    expect(html).toContain("Edit document paragraph");
    expect(html).toContain("Existing site area");
    expect(html).toContain("Create edit candidate");
  });

  it("renders a candidate editor bound to the exact semantic table-cell ID", () => {
    const html = renderToStaticMarkup(
      <DocumentTableCellEditor
        client={{
          proposeDocumentTableCellEdit: async () => ({
            project: "project-1",
            candidateId: "candidate-cell-1",
            baseSnapshot: "snapshot-1",
            path: "report.docx",
            validation: { status: "pending" },
          }),
        }}
        project="project-1"
        snapshot="snapshot-1"
        cell="cell-1"
        text="parcel 124"
      />,
    );
    expect(html).toContain("Edit document table cell");
    expect(html).toContain("parcel 124");
    expect(html).toContain("Create edit candidate");
  });

  it("renders document paragraph and table identities from the semantic tree", () => {
    const tree = {
      project: "project-1",
      snapshot: "snapshot-1",
      document: "document-1",
      artifactCapability: "editable_with_opaque_preservation" as const,
      sections: [
        {
          id: "section-1",
          blocks: [
            {
              kind: "paragraph" as const,
              paragraph: {
                id: "paragraph-1",
                runs: [{ id: "run-1", text: "Editable paragraph" }],
              },
            },
            {
              kind: "table" as const,
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
                            kind: "paragraph" as const,
                            paragraph: {
                              id: "complex-1",
                              runs: [
                                { id: "run-a", text: "Protected " },
                                { id: "run-b", text: "formatting" },
                              ],
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
    const controller = {
      currentTree: () => tree,
      read: async () => tree,
      replaceParagraph: async () => ({
        project: "project-1",
        candidateId: "candidate-1",
        baseSnapshot: "snapshot-1",
        path: "report.docx",
        validation: { status: "pending" as const },
      }),
    };
    const html = renderToStaticMarkup(
      <DocumentTreeView
        controller={controller as never}
        client={{} as never}
        onSelectParagraph={() => undefined}
        onSelectTableCell={() => undefined}
      />,
    );
    expect(html).toContain('data-paragraph-id="paragraph-1"');
    expect(html).toContain('data-table-id="table-1"');
    expect(html).toContain('data-cell-id="cell-1"');
    expect(html).toContain("Select paragraph paragraph-1");
    expect(html).toContain("Select table cell cell-1");
    expect(html).toContain("Editable paragraph");
    expect(html).toContain("Create edit candidate");
    expect(html).toContain("This paragraph structure is currently read-only.");
    expect(html).not.toContain("word/document.xml");
  });

  it("offers stable cell editing for a structurally simple DOCX table cell", () => {
    const tree = {
      project: "project-1",
      snapshot: "snapshot-1",
      document: "document-1",
      artifactCapability: "editable_with_opaque_preservation" as const,
      sections: [
        {
          id: "section-1",
          blocks: [
            {
              kind: "table" as const,
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
                            kind: "paragraph" as const,
                            paragraph: {
                              id: "paragraph-1",
                              runs: [{ id: "run-1", text: "parcel 124" }],
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
    const controller = {
      currentTree: () => tree,
      read: async () => tree,
      replaceTableCell: async () => ({
        project: "project-1",
        candidateId: "candidate-cell-1",
        baseSnapshot: "snapshot-1",
        path: "report.docx",
        validation: { status: "pending" as const },
      }),
    };
    const html = renderToStaticMarkup(
      <DocumentTreeView controller={controller as never} client={{} as never} />,
    );
    expect(html).toContain('data-cell-id="cell-1"');
    expect(html).toContain("Edit document table cell");
    expect(html).toContain("parcel 124");
  });

  it("keeps document editing disabled when the artifact capability is read-only", () => {
    const tree = {
      project: "project-1",
      snapshot: "snapshot-1",
      document: "document-1",
      artifactCapability: "read_only_derived" as const,
      sections: [
        {
          id: "section-1",
          blocks: [
            {
              kind: "paragraph" as const,
              paragraph: { id: "paragraph-1", runs: [{ id: "run-1", text: "Read only" }] },
            },
          ],
        },
      ],
    };
    const controller = { currentTree: () => tree, read: async () => tree };

    const html = renderToStaticMarkup(
      <DocumentTreeView controller={controller as never} client={{} as never} />,
    );

    expect(html).toContain("This paragraph structure is currently read-only.");
    expect(html).not.toContain("Create edit candidate");
  });

  it("renders artifact fidelity and caps visible member rows", () => {
    const members = Array.from({ length: 21 }, (_, index) => ({
      key: `member-${index}`,
      projectPath: `part-${index}.bin`,
      digest: `digest-${index}`,
    }));
    const html = renderToStaticMarkup(
      <ArtifactInspector
        artifact={{
          id: "artifact-1",
          logicalKey: "roads",
          kind: "compound",
          adapter: "shapefile",
          adapterVersion: "2.1",
          scanId: "scan-1",
          baselineDigest: "baseline-1",
          observedDigest: "observed-1",
          synchronizationStatus: "modified",
          fidelity: "read_only",
          opaquePreservation: "preserved",
          references: [],
          members,
          memberCount: 21,
        }}
      />,
    );
    expect(html).toContain("read_only");
    expect(html).toContain("shapefile 2.1");
    expect(html).toContain("Materialization baseline");
    expect(html).toContain("baseline-1");
    expect(html).toContain("modified");
    expect(html).toContain("observed-1");
    expect(html).toContain("Showing 20 of 21 members");
    expect(html).not.toContain("member-20");
  });

  it("renders only the supplied sparse spreadsheet page with semantic selection", () => {
    const selected: string[] = [];
    const html = renderToStaticMarkup(
      <SpreadsheetCellTable
        page={{
          project: "project-1",
          snapshot: "snapshot-1",
          worksheet: "sheet-1",
          cells: [
            {
              id: "cell-1",
              row: 3,
              column: 28,
              value: { kind: "formula", value: { expression: "=A3*2" } },
            },
          ],
          total: 30,
          hasMore: true,
        }}
        onSelectCell={(cellId) => selected.push(cellId)}
      />,
    );
    expect(html).toContain("AB3");
    expect(html).toContain("=A3*2");
    expect(html).toContain('data-cell-id="cell-1"');
    expect(html).toContain("More cells are available");
    expect(html).not.toContain("cell-2");
    expect(selected).toEqual([]);
  });

  it("renders an explicit bounded spreadsheet viewport control", () => {
    const html = renderToStaticMarkup(
      <SpreadsheetPageView
        controller={{ currentPage: () => undefined } as never}
        viewportBounds={{
          firstRow: 1,
          firstColumn: 1,
          lastRow: 1_048_576,
          lastColumn: 16_384,
        }}
        viewportRows={40}
        prefetchRows={20}
      />,
    );
    expect(html).toContain('aria-label="Spreadsheet row viewport"');
    expect(html).toContain("First visible row");
    expect(html).toContain("Load row window");
    expect(html).toContain("Shows up to 40 rows with 20 rows of prefetch.");
  });

  it("renders historical inspection state", () => {
    const html = renderToStaticMarkup(
      <ObjectInspector
        value={{
          summary: {
            ref: { entityId: "e" as never, snapshotId: "s" as never },
            typeId: "type" as never,
          },
          state: "historical",
        }}
      />,
    );
    expect(html).toContain("historical");
  });

  it("renders nested branches without transcripts", () => {
    const html = renderToStaticMarkup(
      <SessionTree
        branches={[{ id: "b", title: "Branch", children: [{ id: "c", title: "Child" }] }]}
      />,
    );
    expect(html).toContain("Child");
  });

  it("keeps operation display independent from unmount cancellation", () => {
    const html = renderToStaticMarkup(<OperationView id="op" status="running" />);
    expect(html).toContain("Status: running");
  });

  it("shows unsupported projections explicitly", () => {
    const html = renderToStaticMarkup(<ProjectionHost projectionId="missing" handlers={[]} />);
    expect(html).toContain("Unsupported projection");
  });

  it("renders structured semantic migration changes", () => {
    const html = renderToStaticMarkup(
      <MigrationPreview
        baseSnapshot="base"
        changes={[{ entityId: "entity-1" as never, kind: "modified" }]}
        validation="current"
      />,
    );
    expect(html).toContain("modified: entity-1");
  });
});
