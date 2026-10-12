/** @file Verifies accessible, styling-neutral component behavior. */
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  MigrationPreview,
  ObjectInspector,
  OperationView,
  ProjectionHost,
  SessionTree,
} from "../src";

describe("neutral components", () => {
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
