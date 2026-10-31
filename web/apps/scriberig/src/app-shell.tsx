/** @file Defines the public-package workbench shell. */
import { AppBar, Box, Button, Toolbar, Typography } from "@mui/material";
import type { ReactElement, ReactNode } from "react";

/** Renders application navigation and the active view. */
/** @returns The rendered AppShell view or runtime value. */
export function AppShell({
  active,
  onNavigate,
  children,
  status,
  workspace,
}: {
  active: string;
  onNavigate: (route: string) => void;
  children: ReactNode;
  status: string;
  workspace?: { project: { name: string }; snapshot: string };
}): ReactElement {
  const routes = [
    "projects",
    "artifacts",
    "document",
    "cad",
    "spreadsheet",
    "geo",
    "raw",
    "semantic",
    "projection",
    "history",
    "session",
    "migration",
    "operations",
  ];
  return (
    <Box>
      <AppBar position="static">
        <Toolbar>
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            Scriberig
          </Typography>
          {routes.map((route) => (
            <Button
              key={route}
              color={active === route ? "secondary" : "inherit"}
              onClick={() => onNavigate(route)}
            >
              {route}
            </Button>
          ))}
        </Toolbar>
      </AppBar>
      <Box component="p" role="status" sx={{ px: 2 }}>
        {status}
        {workspace ? ` | project: ${workspace.project.name} | snapshot: ${workspace.snapshot}` : ""}
      </Box>
      <Box component="main" sx={{ p: 2 }}>
        {children}
      </Box>
    </Box>
  );
}
