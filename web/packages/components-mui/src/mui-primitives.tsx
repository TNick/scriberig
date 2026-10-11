/** @file Shared Material UI presentation helpers. */
import { Paper, Typography, Alert } from "@mui/material";
import type { ReactElement, ReactNode } from "react";
/** Renders the MuiPanel Material UI component. */
export function MuiPanel({
  title,
  children,
  className,
}: {
  title: string;
  children: ReactNode;
  className?: string | undefined;
}): ReactElement {
  return (
    <Paper component="section" className={className} aria-label={title} sx={{ p: 2 }}>
      <Typography component="h2" variant="h6">
        {title}
      </Typography>
      {children}
    </Paper>
  );
}
/** Renders the MuiStatus Material UI component. */
export function MuiStatus({
  children,
  severity = "info",
}: {
  children: ReactNode;
  severity?: "info" | "success" | "warning" | "error";
}): ReactElement {
  return (
    <Alert severity={severity} role={severity === "error" ? "alert" : "status"}>
      {children}
    </Alert>
  );
}
