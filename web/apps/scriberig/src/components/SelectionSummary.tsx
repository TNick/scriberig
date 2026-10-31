/** @file Renders the shared semantic selection as concise workspace context. */
import { Box, Chip, Typography } from "@mui/material";
import type { ReactElement } from "react";
import type { Selection } from "@scriberig/editor-core";

/** Displays a selected semantic handle and its exact snapshot identity.
 * @param props Current selection, if one exists.
 * @returns A compact selection context strip.
 */
export function SelectionSummary({
  selection,
}: {
  selection?: Selection | undefined;
}): ReactElement {
  return (
    <Box
      role="status"
      aria-label="Semantic selection"
      sx={{
        display: "flex",
        alignItems: "center",
        gap: 1,
        flexWrap: "wrap",
        px: 1.5,
        py: 1,
        mb: 2.5,
        borderLeft: 3,
        borderColor: selection ? "secondary.main" : "divider",
        bgcolor: "background.paper",
        borderRadius: "0 8px 8px 0",
      }}
    >
      <Typography variant="caption" color="text.secondary">
        Selection
      </Typography>
      {selection ? (
        <>
          <Chip size="small" color="secondary" label={selection.entityId} />
          <Typography variant="caption" color="text.secondary">
            at snapshot {selection.snapshotId}
          </Typography>
        </>
      ) : (
        <Typography variant="body2" color="text.secondary">
          Choose an item to carry its context between views.
        </Typography>
      )}
    </Box>
  );
}
