/** @file Provides structured session and agent controls. */
import { useScriberig } from "@scriberig/react";
import { MuiAgentPanel, MuiSessionTree } from "@scriberig/components-mui";
import type { BranchSummary } from "@scriberig/components";
import { useState, type ReactElement } from "react";
/** Renders structured semantic scope and public commands. */
/** @returns The rendered SessionAgentView view or runtime value. */
export function SessionAgentView({
  branches = [],
}: {
  branches?: readonly BranchSummary[];
}): ReactElement {
  const { client, selection, commands, model } = useScriberig();
  const [status, setStatus] = useState("Ready");
  const [output, setOutput] = useState<string>();

  const submit = async (prompt: string) => {
    const kind = commands.find((command) => command.id === prompt)?.execute;
    if (!kind) {
      setStatus(`Unknown command: ${prompt}`);
      return;
    }
    setStatus("Submitting");
    try {
      const operation = await client.command(kind, { prompt, selection: selection.get() });
      model.operations.set(operation);
      setStatus(operation.status);
      setOutput(operation.resultSnapshotId ?? operation.error);
    } catch {
      setStatus("Command failed");
    }
  };

  return (
    <section aria-label="Session and Agent">
      <h1>Session / Agent</h1>
      <MuiSessionTree branches={branches} />
      <MuiAgentPanel
        selection={selection.get()}
        commands={commands}
        status={status}
        {...(output === undefined ? {} : { output })}
        onSubmit={(prompt) => void submit(prompt)}
      />
    </section>
  );
}
