/** @file Defines the top-level reference Scriberig editor component. */

import { ThemeProvider } from "@mui/material/styles";
import CssBaseline from "@mui/material/CssBaseline";
import { Paper, Typography, useMediaQuery } from "@mui/material";
import { useEffect, useMemo, useState, useSyncExternalStore, type ReactElement } from "react";
import { ScriberigProvider } from "@scriberig/react";
import { useSelection, useSnapshot } from "@scriberig/react";
import type { ContextualActionDescriptor, ContextualActionExecutor } from "@scriberig/editor-core";
import { createScriberigTheme } from "@scriberig/components-mui";
import { AppShell, type AppearanceSettings } from "./app-shell";
import { createRuntime } from "./runtime";
import { parseRoute, pathnameForRoute, routeFromPathname, type AppRoute } from "./routes";
import { SnapshotStatus } from "./components/SnapshotStatus";
import { SelectionSummary } from "./components/SelectionSummary";
import type { AppRuntime } from "./runtime";
import { AppRouteContent } from "./AppRouteContent";
import { ContextualHelper } from "./ContextualHelper";

const appearanceKey = "scriberig.appearance.v1";
const defaultAppearance: AppearanceSettings = {
  density: "comfortable",
  textSize: "medium",
  themeMode: "system",
};

/** Loads and validates the user's saved appearance choices.
 * @returns A valid saved preference or the app defaults.
 */
function loadAppearance(): AppearanceSettings {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(appearanceKey) ?? "null");
    if (typeof value !== "object" || value === null) return defaultAppearance;
    const saved = value as Partial<AppearanceSettings>;
    return {
      density:
        saved.density === "compact" || saved.density === "spacious" ? saved.density : "comfortable",
      textSize:
        saved.textSize === "small" || saved.textSize === "large" ? saved.textSize : "medium",
      themeMode:
        saved.themeMode === "light" || saved.themeMode === "dark" ? saved.themeMode : "system",
    };
  } catch {
    return defaultAppearance;
  }
}

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

/** Explains the context needed before a domain view can be opened.
 * @param props Plain-language guidance for the current empty view.
 * @returns A reusable contextual empty state.
 */
function WorkspacePrompt({ children }: { children: string }): ReactElement {
  return (
    <Paper variant="outlined" sx={{ p: { xs: 2, sm: 3 }, maxWidth: 760 }}>
      <Typography variant="h6" sx={{ mb: 0.5 }}>
        Start with a workspace
      </Typography>
      <Typography color="text.secondary">{children}</Typography>
    </Paper>
  );
}

/** Renders the integrated Scriberig application.
 *
 * @returns The rendered application view.
 * @throws Browser storage and history failures are handled at their call sites.
 */
export function App(): ReactElement {
  const runtime = useMemo(() => createRuntime(), []);
  const [route, setRoute] = useState<AppRoute>(() => routeFromPathname(window.location.pathname));
  const [reviewMigrationId, setReviewMigrationId] = useState<string>();
  const [connection, setConnection] = useState(runtime.connection);
  const [workspace, setWorkspace] = useState(runtime.activeWorkspace);
  const [appearance, setAppearance] = useState(loadAppearance);
  const systemPrefersDark = useMediaQuery("(prefers-color-scheme: dark)");
  const resolvedMode =
    appearance.themeMode === "system"
      ? systemPrefersDark
        ? "dark"
        : "light"
      : appearance.themeMode;
  const selection = useRuntimeSelection(runtime);
  const actionContextVersion = useMemo(
    () =>
      `${route}:${workspace?.project.id ?? "none"}:${selection?.entityId ?? "none"}:${selection?.snapshotId ?? "none"}`
        .split("")
        .reduce((hash, character) => (hash * 31 + character.charCodeAt(0)) >>> 0, 0),
    [route, selection, workspace],
  );
  const contextualActions = useMemo<readonly ContextualActionDescriptor[]>(
    () =>
      [
        { id: "navigate", label: "Navigate to another page", mutating: false },
        { id: "inspect-selection", label: "Inspect the current selection", mutating: false },
        { id: "refresh-context", label: "Refresh page context", mutating: false },
        { id: "apply-reviewed-change", label: "Apply a reviewed change", mutating: true },
      ].map((action) => ({ ...action, contextVersion: actionContextVersion })),
    [actionContextVersion],
  );
  const executeContextualAction = useMemo<ContextualActionExecutor>(
    () => async (request) => {
      if (request.actionId === "refresh-context") {
        await runtime.bootstrap();
        return {
          invocationId: request.invocationId,
          ok: true,
          summary: "Page context refreshed.",
          contextVersion: request.contextVersion,
        };
      }
      if (request.actionId === "inspect-selection") {
        return {
          invocationId: request.invocationId,
          ok: true,
          summary: selection === undefined ? "No selection is active." : "Selection is active.",
          contextVersion: request.contextVersion,
        };
      }
      return {
        invocationId: request.invocationId,
        ok: false,
        summary: "This action requires the existing page review controls.",
        contextVersion: request.contextVersion,
      };
    },
    [runtime, selection],
  );
  useEffect(() => {
    void runtime.bootstrap().then(
      () => setConnection(runtime.connection),
      () => setConnection(runtime.connection),
    );
  }, [runtime]);
  useEffect(() => runtime.model.subscribe(() => setConnection(runtime.connection)), [runtime]);
  useEffect(() => runtime.subscribe(() => setWorkspace(runtime.activeWorkspace)), [runtime]);
  useEffect(() => {
    const syncRoute = () => {
      const next = routeFromPathname(window.location.pathname);
      if (window.location.pathname !== "/" && window.location.pathname !== pathnameForRoute(next)) {
        window.history.replaceState(null, "", pathnameForRoute(next));
      }
      setRoute(next);
    };
    window.addEventListener("popstate", syncRoute);
    syncRoute();
    return () => window.removeEventListener("popstate", syncRoute);
  }, []);
  /** Updates the visible route and browser history together.
   * @param value Requested route identifier.
   * @returns Nothing.
   * @throws The route parser is total and does not throw.
   */
  function navigateRoute(value: string): void {
    const next = parseRoute(value);
    const path = pathnameForRoute(next);
    if (window.location.pathname !== path) window.history.pushState(null, "", path);
    setRoute(next);
  }
  return (
    <ThemeProvider
      theme={createScriberigTheme({
        palette: { mode: resolvedMode },
        spacing: { compact: 6, comfortable: 8, spacious: 10 }[appearance.density],
        typography: { fontSize: { small: 14, medium: 16, large: 18 }[appearance.textSize] },
      })}
    >
      <CssBaseline />
      <ScriberigProvider value={runtime}>
        <AppShell
          active={route}
          onNavigate={navigateRoute}
          status={connection}
          appearance={appearance}
          onAppearanceChange={(next) => {
            setAppearance(next);
            try {
              localStorage.setItem(appearanceKey, JSON.stringify(next));
            } catch {
              // Preferences still apply for this session if storage is unavailable.
            }
          }}
          {...(workspace === undefined ? {} : { workspace })}
          helper={
            <ContextualHelper
              route={route}
              workspace={workspace}
              selection={selection}
              actions={contextualActions}
              executeAction={executeContextualAction}
            />
          }
        >
          {workspace !== undefined && <RuntimeSnapshotStatus connection={connection} />}
          <SharedSelectionSummary />
          <AppRouteContent
            route={route}
            runtime={runtime}
            workspace={workspace}
            selection={selection}
            navigate={navigateRoute}
            reviewMigrationId={reviewMigrationId}
            onReviewMigration={(migrationId) => {
              setReviewMigrationId(migrationId);
              navigateRoute("projection");
            }}
            workspacePrompt={(message) => <WorkspacePrompt>{message}</WorkspacePrompt>}
          />
        </AppShell>
      </ScriberigProvider>
    </ThemeProvider>
  );
}
