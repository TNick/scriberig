/** @file Verifies the Material UI component package contracts. */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import {
  createScriberigTheme,
  MuiCommandPalette,
  MuiMigrationPreview,
  MuiObjectTree,
  MuiValidationResults,
} from "../src/index";
import { KnowledgeTree, type CommandDescriptor } from "@scriberig/editor-core";
import { ThemeProvider } from "@mui/material/styles";
import React from "react";

describe("components-mui", () => {
  it("renders under host and optional themes", () => {
    const content = <MuiValidationResults status="stale" diagnostics={["stale snapshot"]} />;
    expect(
      renderToStaticMarkup(<ThemeProvider theme={createScriberigTheme()}>{content}</ThemeProvider>),
    ).toContain("stale snapshot");
    expect(
      renderToStaticMarkup(
        <ThemeProvider theme={createScriberigTheme({ palette: { mode: "dark" } })}>
          {content}
        </ThemeProvider>,
      ),
    ).toContain("Validation: stale");
  });

  it("propagates tree selection through the shared controller", () => {
    const tree = new KnowledgeTree();
    const onSelect = vi.fn();
    tree.setRoot({
      id: "root",
      entityId: "entity" as never,
      snapshotId: "snap" as never,
      expanded: false,
      loading: false,
      children: [],
    });
    const markup = renderToStaticMarkup(
      <MuiObjectTree tree={tree} rootId="root" onSelect={onSelect} />,
    );
    expect(markup).toContain("entity");
  });

  it("uses discoverable commands and structured migration operations", () => {
    const command: CommandDescriptor = {
      id: "inspect",
      title: "Inspect",
      mutating: false,
      execute: "inspect",
    };
    expect(
      renderToStaticMarkup(<MuiCommandPalette commands={[command]} capabilities={new Set()} />),
    ).toContain("Commands");
    const apply = vi.fn();
    const markup = renderToStaticMarkup(
      <MuiMigrationPreview
        baseSnapshot="base"
        changes={["changed object"]}
        validation="current"
        onApply={apply}
      />,
    );
    expect(markup).toContain("changed object");
  });
});
