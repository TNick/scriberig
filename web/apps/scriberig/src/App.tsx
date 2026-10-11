/** @file Defines the top-level reference Scriberig editor component. */

import { ThemeProvider } from "@mui/material/styles";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import { ScriberigProvider } from "@scriberig/react";
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

/**
/** @returns The rendered App view or runtime value. */
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
          <SnapshotStatus state={connection === "connected" ? "current" : connection} />
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
          ) : (
            <OperationsView />
          )}
        </AppShell>
      </ScriberigProvider>
    </ThemeProvider>
  );
}
