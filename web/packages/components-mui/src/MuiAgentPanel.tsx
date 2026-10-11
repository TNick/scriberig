/** @file Presents structured agent scope and commands with Material UI. */
import type { ReactElement } from "react";
import { Chip, TextField, Button, Typography } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { CommandDescriptor, Selection } from "@scriberig/editor-core";
/** Renders the MuiAgentPanel Material UI component. */
export function MuiAgentPanel({
  sessionId,
  branchId,
  selection,
  commands,
  status,
  output,
  resultLinks,
  onSubmit,
  className,
}: {
  sessionId?: string;
  branchId?: string;
  selection?: Selection;
  commands: readonly CommandDescriptor[];
  status: string;
  output?: string;
  resultLinks?: readonly string[];
  onSubmit?: (prompt: string, selection?: Selection) => void;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title="Agent panel" className={className}>
      {sessionId ? <Chip label={`Session ${sessionId}`} /> : null}
      {branchId ? <Chip label={`Branch ${branchId}`} /> : null}
      <Typography>
        {selection ? `Semantic scope: ${JSON.stringify(selection)}` : "No semantic scope selected"}
      </Typography>
      <Typography role="status">{status}</Typography>
      {output ? <Typography component="pre">{output}</Typography> : null}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          const d = new FormData(e.currentTarget);
          onSubmit?.(String(d.get("prompt") ?? ""), selection);
        }}
      >
        <TextField name="prompt" label="Command or instruction" />
        <Button type="submit">Send</Button>
      </form>
      {commands.map((c) => (
        <Chip key={c.id} label={`${c.title}${c.mutating ? " (mutating)" : ""}`} />
      ))}
      {resultLinks?.map((l) => (
        <Typography key={l}>Result: {l}</Typography>
      ))}
    </MuiPanel>
  );
}
