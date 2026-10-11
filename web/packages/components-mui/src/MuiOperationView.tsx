/** @file Renders durable operation progress with Material UI. */
import type { ReactElement } from "react";
import { LinearProgress, Typography } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
/** Renders the MuiOperationView Material UI component. */
export function MuiOperationView({
  id,
  status,
  progress,
  result,
  className,
}: {
  id: string;
  status: string;
  progress?: number;
  result?: string;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title={`Operation ${id}`} className={className}>
      <Typography role="status">Status: {status}</Typography>
      {progress !== undefined ? <LinearProgress variant="determinate" value={progress} /> : null}
      {result ? <Typography>Result: {result}</Typography> : null}
    </MuiPanel>
  );
}
