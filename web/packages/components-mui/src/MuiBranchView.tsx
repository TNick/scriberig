/** @file Renders one session branch summary with Material UI. */
import type { ReactElement } from "react";
import { Chip, Typography } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { BranchSummary } from "@scriberig/components";
/** Renders the MuiBranchView Material UI component. */
export function MuiBranchView({
  branch,
  className,
}: {
  branch: BranchSummary;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title={`Branch ${branch.title}`} className={className}>
      {branch.status ? <Chip label={branch.status} /> : null}
      {branch.children?.length ? (
        <Typography>{branch.children.length} child branches</Typography>
      ) : null}
    </MuiPanel>
  );
}
