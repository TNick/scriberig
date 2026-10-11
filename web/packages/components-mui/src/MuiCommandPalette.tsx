/** @file Provides searchable invocation of public editor commands. */
import { useMemo, useState, type ReactElement } from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogTitle,
  List,
  ListItemButton,
  ListItemText,
  TextField,
} from "@mui/material";
import { discoverCommands, type CommandDescriptor, type Selection } from "@scriberig/editor-core";
/** Renders the MuiCommandPalette Material UI component. */
export function MuiCommandPalette({
  commands,
  selection,
  capabilities,
  onExecute,
  className,
}: {
  commands: readonly CommandDescriptor[];
  selection?: Selection;
  capabilities: ReadonlySet<string>;
  onExecute?: (c: CommandDescriptor) => void;
  className?: string;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const a = useMemo(
    () =>
      discoverCommands(commands, selection, capabilities).filter((c) =>
        `${c.id} ${c.title}`.toLowerCase().includes(q.toLowerCase()),
      ),
    [commands, selection, capabilities, q],
  );
  return (
    <span className={className}>
      <Button aria-label="Open command palette" onClick={() => setOpen(true)}>
        Commands
      </Button>
      <Dialog open={open} onClose={() => setOpen(false)}>
        <DialogTitle>Command palette</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            label="Search commands"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
          <List>
            {a.map((c) => (
              <ListItemButton
                key={c.id}
                onClick={() => {
                  onExecute?.(c);
                  setOpen(false);
                }}
              >
                <ListItemText
                  primary={c.title}
                  secondary={c.mutating ? "Mutating command" : "Read-only command"}
                />
              </ListItemButton>
            ))}
          </List>
        </DialogContent>
      </Dialog>
    </span>
  );
}
