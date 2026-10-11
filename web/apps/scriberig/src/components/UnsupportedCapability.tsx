/** @file Renders explicit unsupported capability state. */
import { Alert } from "@mui/material";
import type { ReactElement } from "react";

/** Informs users when a server capability is unavailable. */
/** @returns The rendered UnsupportedCapability view or runtime value. */
export function UnsupportedCapability({ capability }: { capability: string }): ReactElement {
  return (
    <Alert severity="info" role="status">
      Unsupported capability: {capability}
    </Alert>
  );
}
