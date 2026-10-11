/** @file Renders semantic object inspection with Material UI surfaces. */
import type { ReactElement } from "react";
import { Chip, Divider, List, ListItem, Typography } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { InspectorValue } from "@scriberig/components";
/** Renders the MuiObjectInspector Material UI component. */
export function MuiObjectInspector({
  value,
  className,
}: {
  value: InspectorValue;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title="Object inspector" className={className}>
      <Typography>{value.summary.label ?? value.summary.ref.entityId}</Typography>
      <Typography variant="body2">
        {value.summary.typeId} Â· {value.summary.ref.revisionId ?? "revision unavailable"}
      </Typography>
      {value.state && value.state !== "current" ? <Chip label={value.state} role="status" /> : null}
      {value.description ? <Typography>{value.description}</Typography> : null}
      {value.provenance ? <Typography>Provenance: {value.provenance}</Typography> : null}
      {value.properties ? (
        <>
          <Divider />
          <List>
            {value.properties.map(([k, v]) => (
              <ListItem key={k}>
                {k}: {v}
              </ListItem>
            ))}
          </List>
        </>
      ) : null}
      {value.relations ? (
        <>
          <Divider />
          <List>
            {value.relations.map((r) => (
              <ListItem key={r}>{r}</ListItem>
            ))}
          </List>
        </>
      ) : null}
    </MuiPanel>
  );
}
