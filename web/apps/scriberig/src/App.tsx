/** @file Defines the top-level reference Scriberig editor component. */

import { ThemeProvider } from "@mui/material/styles";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import { ScriberigProvider } from "@scriberig/react";
import { useSelection, useSnapshot } from "@scriberig/react";
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
  const [route, setRoute] = useState<AppRoute>("raw");
  const [reviewMigrationId, setReviewMigrationId] = useState<string>();
  const [connection, setConnection] = useState(runtime.connection);
  useEffect(() => {
    void runtime.bootstrap().then(
      () => setConnection(runtime.connection),
      () => setConnection(runtime.connection),
    );
  }, [runtime]);
  useEffect(() => runtime.model.subscribe(() => setConnection(runtime.connection)), [runtime]);
  return (
    <ThemeProvider theme={createScriberigTheme()}>
      <ScriberigProvider value={runtime}>
        <AppShell
          active={route}
          onNavigate={(next) => setRoute(parseRoute(next))}
          status={connection}
        >
          <RuntimeSnapshotStatus connection={connection} />
          <SharedSelectionSummary />
          {route === "raw" ? (
            <RawKnowledgeView />
          ) : route === "semantic" ? (
            <SemanticView />
          ) : route === "projection" ? (
            <ProjectionView />
          ) : route === "history" ? (
            <HistoryView />
          ) : route === "session" ? (
            <SessionAgentView
              onReviewCandidate={(migrationId) => {
                setReviewMigrationId(migrationId);
                setRoute("migration");
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
