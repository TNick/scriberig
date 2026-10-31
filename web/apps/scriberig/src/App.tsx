/** @file Defines the top-level reference Scriberig editor component. */

import { ThemeProvider } from "@mui/material/styles";
import { useEffect, useMemo, useState, useSyncExternalStore, type ReactElement } from "react";
import { ScriberigProvider } from "@scriberig/react";
import { useSelection, useSnapshot } from "@scriberig/react";
import {
  selectCadEntity,
  selectDocumentPart,
  selectGeoFeature,
  selectSpreadsheetCell,
  selectSpreadsheetRange,
} from "@scriberig/editor-core";
import type { EntityId, SnapshotId } from "@scriberig/protocol";
import { createScriberigTheme } from "@scriberig/components-mui";
import { AppShell } from "./app-shell";
import { createRuntime } from "./runtime";
import { parseRoute, type AppRoute } from "./routes";
import { SnapshotStatus } from "./components/SnapshotStatus";
import { RawKnowledgeView } from "./views/RawKnowledgeView";
import { SemanticView } from "./views/SemanticView";
import { ProjectionView } from "./views/ProjectionView";
import { HistoryView } from "./views/HistoryView";
import { SessionAgentView } from "./views/SessionAgentView";
import { OperationsView } from "./views/OperationsView";
import { MigrationReviewView } from "./views/MigrationReviewView";
import { SelectionSummary } from "./components/SelectionSummary";
import { ProjectManager } from "./views/ProjectManager";
import { ArtifactsView } from "./views/ArtifactsView";
import { DocumentView } from "./views/DocumentView";
import { CadView } from "./views/CadView";
import { SpreadsheetView } from "./views/SpreadsheetView";
import { GeoView } from "./views/GeoView";
import type { AppRuntime } from "./runtime";

/** Subscribes the application composition root to its owned selection controller.
 * @param runtime Runtime that owns the shared semantic selection.
 * @returns The current selection, if one is active.
 */
function useRuntimeSelection(runtime: AppRuntime) {
  return useSyncExternalStore(
    (listener) => runtime.selection.subscribe(listener),
    () => runtime.selection.get(),
    () => runtime.selection.get(),
  );
}

/** Renders the semantic selection shared by every integrated route.
 * @returns The shared selection summary.
 */
function SharedSelectionSummary(): ReactElement {
  const { selection } = useSelection();
  return <SelectionSummary selection={selection} />;
}

/** Renders the current connection and snapshot status.
 *
 * @param connection Runtime connection state.
 * @returns The rendered snapshot status.
 */
function RuntimeSnapshotStatus({ connection }: { connection: string }): ReactElement {
  const snapshot = useSnapshot();
  return (
    <SnapshotStatus
      state={
        connection === "connected" ? snapshot.kind : (connection as "connecting" | "disconnected")
      }
    />
  );
}

/** Renders the integrated Scriberig application.
 *
 * @returns The rendered application view.
 */
export function App(): ReactElement {
  const runtime = useMemo(() => createRuntime(), []);
  const [route, setRoute] = useState<AppRoute>("projects");
  const [reviewMigrationId, setReviewMigrationId] = useState<string>();
  const [connection, setConnection] = useState(runtime.connection);
  const [workspace, setWorkspace] = useState(runtime.activeWorkspace);
  const selection = useRuntimeSelection(runtime);
  useEffect(() => {
    void runtime.bootstrap().then(
      () => setConnection(runtime.connection),
      () => setConnection(runtime.connection),
    );
  }, [runtime]);
  useEffect(() => runtime.model.subscribe(() => setConnection(runtime.connection)), [runtime]);
  useEffect(() => runtime.subscribe(() => setWorkspace(runtime.activeWorkspace)), [runtime]);
  return (
    <ThemeProvider theme={createScriberigTheme()}>
      <ScriberigProvider value={runtime}>
        <AppShell
          active={route}
          onNavigate={(next) => setRoute(parseRoute(next))}
          status={connection}
          {...(workspace === undefined ? {} : { workspace })}
        >
          <RuntimeSnapshotStatus connection={connection} />
          <SharedSelectionSummary />
          {route === "raw" ? (
            <RawKnowledgeView />
          ) : route === "projects" ? (
            <ProjectManager
              client={runtime.client}
              onActivate={async (projectId) => {
                await runtime.activateProject(projectId);
                setRoute("semantic");
              }}
            />
          ) : route === "artifacts" ? (
            workspace === undefined ? (
              <p>Open a project to inspect its artifacts.</p>
            ) : (
              <ArtifactsView client={runtime.client} project={workspace.project.id} />
            )
          ) : route === "document" ? (
            workspace === undefined || selection === undefined ? (
              <p>Open a project and select a document root to inspect its semantic structure.</p>
            ) : (
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
                  runtime.model.apply({
                    kind: "snapshotAdvanced",
                    snapshotId: snapshot as SnapshotId,
                  });
                  const updatedSelection = { ...selection, snapshotId: snapshot as never };
                  delete updatedSelection.revisionId;
                  delete updatedSelection.anchor;
                  runtime.selection.set(updatedSelection);
                }}
              />
            )
          ) : route === "cad" ? (
            workspace === undefined || selection === undefined ? (
              <p>
                Open a project and select a CAD drawing to inspect its semantic layers and entities.
              </p>
            ) : (
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
                  runtime.model.apply({
                    kind: "snapshotAdvanced",
                    snapshotId: result.successor as never,
                  });
                  const updatedSelection = {
                    ...selection,
                    snapshotId: result.successor as never,
                  };
                  delete updatedSelection.revisionId;
                  delete updatedSelection.anchor;
                  runtime.selection.set(updatedSelection);
                }}
              />
            )
          ) : route === "spreadsheet" ? (
            workspace === undefined || selection === undefined ? (
              <p>Open a project and select a worksheet to inspect its bounded cell pages.</p>
            ) : (
              <SpreadsheetView
                client={runtime.client}
                project={workspace.project.id}
                snapshot={selection.snapshotId}
                worksheet={
                  selection.anchor?.kind === "spreadsheetCell"
                    ? selection.anchor.worksheetId
                    : selection.entityId
                }
                onSelectCell={(cell, snapshot) =>
                  runtime.selection.set(
                    selectSpreadsheetCell(
                      cell.id as EntityId,
                      snapshot as SnapshotId,
                      selection.anchor?.kind === "spreadsheetCell" ||
                        selection.anchor?.kind === "spreadsheetRange"
                        ? selection.anchor.worksheetId
                        : (selection.entityId as EntityId),
                      cell.row,
                      cell.column,
                    ),
                  )
                }
                onSelectRange={(range, snapshot) =>
                  runtime.selection.set(
                    selectSpreadsheetRange(
                      selection.anchor?.kind === "spreadsheetCell" ||
                        selection.anchor?.kind === "spreadsheetRange"
                        ? selection.anchor.worksheetId
                        : (selection.entityId as EntityId),
                      snapshot as SnapshotId,
                      range.firstRow,
                      range.firstColumn,
                      range.lastRow,
                      range.lastColumn,
                    ),
                  )
                }
                onCandidateApplied={(snapshot) => {
                  runtime.model.apply({
                    kind: "snapshotAdvanced",
                    snapshotId: snapshot as SnapshotId,
                  });
                  if (selection !== undefined) {
                    const updatedSelection = {
                      ...selection,
                      snapshotId: snapshot as never,
                    };
                    delete updatedSelection.revisionId;
                    runtime.selection.set(updatedSelection);
                  }
                }}
              />
            )
          ) : route === "geo" ? (
            workspace === undefined ? (
              <p>Open a project to inspect its geospatial layers.</p>
            ) : (
              <GeoView
                client={runtime.client}
                project={workspace.project.id}
                snapshot={workspace.snapshot}
                onSelectFeature={(feature, snapshot, layer) =>
                  runtime.selection.set(
                    selectGeoFeature(
                      feature.id as EntityId,
                      snapshot as SnapshotId,
                      layer as EntityId,
                    ),
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
            )
          ) : route === "semantic" ? (
            <SemanticView onNavigate={setRoute} />
          ) : route === "projection" ? (
            <ProjectionView />
          ) : route === "history" ? (
            <HistoryView />
          ) : route === "session" ? (
            <SessionAgentView
              onReviewCandidate={(migrationId) => {
                setReviewMigrationId(migrationId);
                setRoute("projection");
              }}
            />
          ) : route === "migration" ? (
            <MigrationReviewView
              {...(reviewMigrationId === undefined ? {} : { migrationId: reviewMigrationId })}
            />
          ) : (
            <OperationsView />
          )}
        </AppShell>
      </ScriberigProvider>
    </ThemeProvider>
  );
}
