/** @file Renders the contextual helper launcher and responsive conversation panel. */

import {
  Box,
  Dialog,
  DialogContent,
  DialogTitle,
  Fab,
  IconButton,
  Tooltip,
  Typography,
  useMediaQuery,
  useTheme,
} from "@mui/material";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import {
  ContextualActionRegistry,
  type ContextualActionDescriptor,
  type ContextualActionExecutor,
  type ContextualActionRequest,
} from "@scriberig/editor-core";
import type { AppRuntime } from "./runtime";
import type { AppRoute } from "./routes";
import { SessionAgentView } from "./views/SessionAgentView";

/** Renders the contextual helper available from every route.
 * @param props Current route, workspace, selection, and action bridge.
 * @returns The helper launcher and responsive conversation panel.
 */
export function ContextualHelper({
  route,
  workspace,
  selection,
  actions,
  executeAction,
}: {
  route: AppRoute;
  workspace: AppRuntime["activeWorkspace"];
  selection: ReturnType<AppRuntime["selection"]["get"]>;
  actions: readonly ContextualActionDescriptor[];
  executeAction: ContextualActionExecutor;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [configurationOpen, setConfigurationOpen] = useState(false);
  const [profile, setProfile] = useState(() => {
    try {
      return globalThis.localStorage?.getItem("scriberig.helper.profile") ?? "contextual-helper";
    } catch {
      return "contextual-helper";
    }
  });
  const theme = useTheme();
  const mobile = useMediaQuery(theme.breakpoints.down("sm"));
  const actionRegistry = useMemo(() => new ContextualActionRegistry(), []);
  useEffect(() => {
    actionRegistry.set(actions);
    const onRequest = (event: Event): void => {
      const request = (event as CustomEvent<ContextualActionRequest>).detail;
      if (request === undefined || typeof request.actionId !== "string") return;
      const invalid = actionRegistry.validate(request);
      void Promise.resolve(invalid ?? executeAction(request)).then((result) => {
        globalThis.dispatchEvent(
          new CustomEvent("scriberig:contextual-action-result", { detail: result }),
        );
      });
    };
    globalThis.addEventListener("scriberig:contextual-action-request", onRequest);
    return () => globalThis.removeEventListener("scriberig:contextual-action-request", onRequest);
  }, [actionRegistry, actions, executeAction]);
  const project = workspace?.project.name ?? "No workspace open";
  const selectionText =
    selection === undefined
      ? "No semantic selection"
      : `Selected entity ${selection.entityId} at snapshot ${selection.snapshotId}`;
  const actionText = actions
    .map((action) => `${action.id}: ${action.label}${action.mutating ? " (changes data)" : ""}`)
    .join(", ");
  const prompt = `You are helping on the Scriberig ${route} page using the ${profile} profile. Current workspace: ${project}. Current context: ${selectionText}. Available page actions: ${actionText || "none"}. Use the available agent tools to inspect context before acting, preserve snapshot identity, and explain every change. Treat page action context as version ${actions[0]?.contextVersion ?? 0}; if it is stale, ask the user to refresh context.`;
  return (
    <>
      <Tooltip title="Open contextual helper">
        <Fab
          color="primary"
          aria-label="Open contextual helper"
          onClick={() => setOpen(true)}
          sx={{
            position: "fixed",
            right: { xs: 18, sm: 28 },
            bottom: { xs: 18, sm: 28 },
            zIndex: theme.zIndex.fab,
          }}
        >
          ✦
        </Fab>
      </Tooltip>
      <Dialog
        open={open}
        fullScreen={mobile}
        onClose={() => setOpen(false)}
        fullWidth
        maxWidth={maximized ? false : "sm"}
        slotProps={{
          paper: {
            sx: maximized
              ? {
                  width: "90vw",
                  minWidth: "90vw",
                  height: "90vh",
                  minHeight: "90vh",
                  maxWidth: "90vw",
                  maxHeight: "90vh",
                  margin: 0,
                }
              : { height: mobile ? "100%" : "min(680px, 82vh)" },
          },
        }}
      >
        <DialogTitle
          sx={{
            display: "flex",
            alignItems: "center",
            gap: 1,
            borderBottom: 1,
            borderColor: "divider",
          }}
        >
          <Box sx={{ flex: 1 }}>
            <Typography variant="h6">Page helper</Typography>
            <Typography variant="caption" color="text.secondary">
              {route} · {project}
            </Typography>
          </Box>
          <Tooltip title="Agent settings">
            <IconButton
              aria-label="Agent settings"
              onClick={() => setConfigurationOpen((current) => !current)}
            >
              ⚙
            </IconButton>
          </Tooltip>
          {!mobile && (
            <Tooltip title={maximized ? "Restore helper" : "Maximize helper"}>
              <IconButton
                aria-label={maximized ? "Restore helper" : "Maximize helper"}
                onClick={() => setMaximized((current) => !current)}
              >
                ⛶
              </IconButton>
            </Tooltip>
          )}
          <IconButton aria-label="Close contextual helper" onClick={() => setOpen(false)}>
            ×
          </IconButton>
        </DialogTitle>
        <DialogContent sx={{ p: 0, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <Box
            sx={{
              display: configurationOpen ? "block" : "none",
              p: 2,
              borderBottom: 1,
              borderColor: "divider",
              bgcolor: "action.hover",
            }}
          >
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              Helper configuration
            </Typography>
            <label>
              Agent profile{" "}
              <select
                aria-label="Agent profile"
                value={profile}
                onChange={(event) => {
                  const next = event.currentTarget.value;
                  setProfile(next);
                  try {
                    globalThis.localStorage?.setItem("scriberig.helper.profile", next);
                  } catch {
                    /* Storage is optional. */
                  }
                }}
              >
                <option value="contextual-helper">Contextual helper</option>
                <option value="project-editor">Project editor</option>
                <option value="researcher">Researcher</option>
              </select>
            </label>
            <Typography variant="body2" color="text.secondary">
              Choose the profile, provider, model, thinking level, and presentation controls for
              this shared workspace conversation.
            </Typography>
          </Box>
          <Box sx={{ p: 2, overflow: "auto", flex: 1 }}>
            <SessionAgentView
              contextPrompt={prompt}
              sessionScope={workspace?.project.id ?? "application"}
              showConfiguration={configurationOpen}
            />
          </Box>
        </DialogContent>
      </Dialog>
    </>
  );
}
