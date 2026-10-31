/** @file Renders route-specific content for the integrated Scriberig shell. */

import type { ReactElement } from "react";
import {
  selectCadEntity,
  selectDocumentPart,
  selectGeoFeature,
  selectSpreadsheetCell,
  selectSpreadsheetRange,
} from "@scriberig/editor-core";
import type { EntityId, SnapshotId } from "@scriberig/protocol";
import type { AppRuntime } from "./runtime";
import type { AppRoute } from "./routes";
import { RawKnowledgeView } from "./views/RawKnowledgeView";
import { ProjectManager } from "./views/ProjectManager";
import { ArtifactsView } from "./views/ArtifactsView";
import { DocumentView } from "./views/DocumentView";
import { CadView } from "./views/CadView";
import { SpreadsheetView } from "./views/SpreadsheetView";
import { GeoView } from "./views/GeoView";
import { SemanticView } from "./views/SemanticView";
import { ProjectionView } from "./views/ProjectionView";
import { HistoryView } from "./views/HistoryView";
import { SessionAgentView } from "./views/SessionAgentView";
import { MigrationReviewView } from "./views/MigrationReviewView";
import { OperationsView } from "./views/OperationsView";

/** Renders a route's page content over shared runtime state.
 * @param props Route content dependencies.
 * @returns The active page.
 */
export function AppRouteContent({
  route,
  runtime,
  workspace,
  selection,
  navigate,
  reviewMigrationId,
  onReviewMigration,
  workspacePrompt,
}: {
  route: AppRoute;
  runtime: AppRuntime;
  workspace: AppRuntime["activeWorkspace"];
  selection: ReturnType<AppRuntime["selection"]["get"]>;
  navigate: (route: string) => void;
  reviewMigrationId: string | undefined;
  onReviewMigration: (migrationId: string) => void;
  workspacePrompt: (message: string) => ReactElement;
}): ReactElement {
  if (route === "raw") return <RawKnowledgeView />;
  if (route === "projects") {
    return (
      <ProjectManager
        client={runtime.client}
        onActivate={async (projectId) => {
          await runtime.activateProject(projectId);
          navigate("semantic");
        }}
      />
    );
  }
  if (route === "artifacts") {
    return workspace === undefined ? (
      workspacePrompt("Open a project to browse its source files and imported artifacts.")
    ) : (
      <ArtifactsView client={runtime.client} project={workspace.project.id} />
    );
  }
  if (route === "document") {
    if (workspace === undefined || selection === undefined) {
      return workspacePrompt(
        "Open a project, then select a document root from Semantic or Artifacts to inspect its structure.",
      );
    }
    return (
      <DocumentView
        client={runtime.client}
        project={workspace.project.id}
        snapshot={selection.snapshotId}
        document={selection.relationPath?.[0] ?? selection.entityId}
        onSelectParagraph={(paragraph, snapshot) =>
          runtime.selection.set(selectDocumentPart(selection, paragraph, snapshot))
        }
        onSelectTableCell={(cell, snapshot) =>
          runtime.selection.set(selectDocumentPart(selection, cell, snapshot))
        }
        onCandidateApplied={(snapshot) => {
          runtime.model.apply({ kind: "snapshotAdvanced", snapshotId: snapshot as SnapshotId });
          const updatedSelection = { ...selection, snapshotId: snapshot as never };
          delete updatedSelection.revisionId;
          delete updatedSelection.anchor;
          runtime.selection.set(updatedSelection);
        }}
      />
    );
  }
  if (route === "cad") {
    if (workspace === undefined || selection === undefined) {
      return workspacePrompt(
        "Open a project, then select a CAD drawing to explore its layers and entities.",
      );
    }
    return (
      <CadView
        client={runtime.client}
        project={workspace.project.id}
        snapshot={selection.snapshotId}
        drawing={selection.relationPath?.[0] ?? selection.entityId}
        onSelectEntity={({ entity, bounds }) => {
          const space =
            entity.space === "Model"
              ? "model"
              : "Paper" in entity.space
                ? `paper:${entity.space.Paper}`
                : `named:${entity.space.Named}`;
          runtime.selection.set(
            selectCadEntity(
              entity.id as EntityId,
              selection.snapshotId,
              selection.entityId as EntityId,
              space,
              bounds,
            ),
          );
        }}
        onCandidateApplied={(result) => {
          runtime.model.apply({ kind: "snapshotAdvanced", snapshotId: result.successor as never });
          const updatedSelection = { ...selection, snapshotId: result.successor as never };
          delete updatedSelection.revisionId;
          delete updatedSelection.anchor;
          runtime.selection.set(updatedSelection);
        }}
      />
    );
  }
  if (route === "spreadsheet") {
    if (workspace === undefined || selection === undefined) {
      return workspacePrompt(
        "Open a project, then select a worksheet to browse its cells and ranges.",
      );
    }
    const worksheet =
      selection.anchor?.kind === "spreadsheetCell"
        ? selection.anchor.worksheetId
        : selection.entityId;
    const worksheetId =
      selection.anchor?.kind === "spreadsheetCell" || selection.anchor?.kind === "spreadsheetRange"
        ? selection.anchor.worksheetId
        : selection.entityId;
    return (
      <SpreadsheetView
        client={runtime.client}
        project={workspace.project.id}
        snapshot={selection.snapshotId}
        worksheet={worksheet}
        onSelectCell={(cell, snapshot) =>
          runtime.selection.set(
            selectSpreadsheetCell(
              cell.id as EntityId,
              snapshot as SnapshotId,
              worksheetId as EntityId,
              cell.row,
              cell.column,
            ),
          )
        }
        onSelectRange={(range, snapshot) =>
          runtime.selection.set(
            selectSpreadsheetRange(
              worksheetId as EntityId,
              snapshot as SnapshotId,
              range.firstRow,
              range.firstColumn,
              range.lastRow,
              range.lastColumn,
            ),
          )
        }
        onCandidateApplied={(snapshot) => {
          runtime.model.apply({ kind: "snapshotAdvanced", snapshotId: snapshot as SnapshotId });
          const updatedSelection = { ...selection, snapshotId: snapshot as never };
          delete updatedSelection.revisionId;
          runtime.selection.set(updatedSelection);
        }}
      />
    );
  }
  if (route === "geo") {
    return workspace === undefined ? (
      workspacePrompt("Open a project to explore its geospatial layers and features.")
    ) : (
      <GeoView
        client={runtime.client}
        project={workspace.project.id}
        snapshot={workspace.snapshot}
        onSelectFeature={(feature, snapshot, layer) =>
          runtime.selection.set(
            selectGeoFeature(feature.id as EntityId, snapshot as SnapshotId, layer as EntityId),
          )
        }
        onCandidateApplied={(snapshot) => {
          runtime.model.apply({ kind: "snapshotAdvanced", snapshotId: snapshot as never });
          if (selection !== undefined) {
            const updatedSelection = { ...selection, snapshotId: snapshot as SnapshotId };
            delete updatedSelection.revisionId;
            delete updatedSelection.anchor;
            runtime.selection.set(updatedSelection);
          }
        }}
      />
    );
  }
  if (route === "semantic") return <SemanticView onNavigate={navigate} />;
  if (route === "projection") return <ProjectionView />;
  if (route === "history") return <HistoryView />;
  if (route === "session") {
    return <SessionAgentView onReviewCandidate={onReviewMigration} />;
  }
  if (route === "migration") {
    return (
      <MigrationReviewView
        {...(reviewMigrationId === undefined ? {} : { migrationId: reviewMigrationId })}
      />
    );
  }
  return <OperationsView />;
}
