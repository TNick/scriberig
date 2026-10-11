/** @file Defines the top-level reference Scriberig editor component. */

import { ThemeProvider } from "@mui/material/styles";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import { ScriberigProvider } from "@scriberig/react";
import { useSnapshot } from "@scriberig/react";
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
  const [connection, setConnection] = useState(runtime.connection);
  useEffect(() => {
    void runtime.bootstrap().then(
      () => setConnection(runtime.connection),
      () => setConnection(runtime.connection),
    );
  }, [runtime]);
  return (
    <ThemeProvider theme={createScriberigTheme()}>
      <ScriberigProvider value={runtime}>
        <AppShell
          active={route}
          onNavigate={(next) => setRoute(parseRoute(next))}
          status={connection}
        >
          <RuntimeSnapshotStatus connection={connection} />
          {route === "raw" ? (
            <RawKnowledgeView />
          ) : route === "semantic" ? (
            <SemanticView />
          ) : route === "projection" ? (
            <ProjectionView />
          ) : route === "history" ? (
            <HistoryView />
          ) : route === "session" ? (
            <SessionAgentView />
          ) : route === "migration" ? (
            <MigrationReviewView />
          ) : (
            <OperationsView />
          )}
        </AppShell>
      </ScriberigProvider>
    </ThemeProvider>
  );
}
