/** @file Provides the MUI wrapper for the public trust summary component. */

import type { ReactElement } from "react";
import { Paper, Typography } from "@mui/material";
import { TrustSummaryView } from "@scriberig/components";
import type { TrustSummary } from "@scriberig/protocol";

/**
 * Wraps the neutral trust summary in MUI presentation.
 * @returns A MUI trust summary element.
 */
export function MuiTrustSummary({ summary }: { summary: TrustSummary }): ReactElement {
  return (
    <Paper elevation={1}>
      <Typography component="h2" variant="h6">
        Trust
      </Typography>
      <TrustSummaryView summary={summary} />
    </Paper>
  );
}
