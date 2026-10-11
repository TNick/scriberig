/** @file Renders the shared semantic selection summary. */
import { Paper, Typography } from "@mui/material";
import type { ReactElement } from "react";
import type { Selection } from "@scriberig/editor-core";

/** Displays semantic handles without converting them to file coordinates. */
/** @returns The rendered SelectionSummary view or runtime value. */
export function SelectionSummary({
  selection,
}: {
  selection?: Selection | undefined;
}): ReactElement {
  return (
    <Paper sx={{ p: 1 }}>
      <Typography variant="body2">
        {selection
          ? `Entity ${selection.entityId} Â· Snapshot ${selection.snapshotId}`
          : "No semantic selection"}
      </Typography>
    </Paper>
  );
}
