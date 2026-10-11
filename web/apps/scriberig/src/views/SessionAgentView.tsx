/** @file Provides structured session and agent controls. */
import { useScriberig } from "@scriberig/react";
import { MuiAgentPanel, MuiSessionTree } from "@scriberig/components-mui";
import type { BranchSummary } from "@scriberig/components";
import type { ReactElement } from "react";
import { UnsupportedCapability } from "../components/UnsupportedCapability";
/** Renders structured semantic scope and public commands. */
/** @returns The rendered SessionAgentView view or runtime value. */
export function SessionAgentView({
  branches = [],
}: {
  branches?: readonly BranchSummary[];
}): ReactElement {
  const { selection, commands } = useScriberig();
  return (
    <section aria-label="Session and Agent">
      <h1>Session / Agent</h1>
      <UnsupportedCapability capability="sessions and agent operations" />
      <MuiSessionTree branches={branches} />
      <MuiAgentPanel selection={selection.get()} commands={commands} status="Ready" />
    </section>
  );
}
