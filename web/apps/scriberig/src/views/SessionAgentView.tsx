/** @file Composes the reusable branch-aware agent workspace view. */

import {
  AgentComposer,
  AgentOperationPanel,
  BranchActivityView,
  SessionBranchTree,
} from "@scriberig/agent-ui";
import { useOperation, useSelection, useScriberig } from "@scriberig/react";
import type { BranchDescriptor, Operation } from "@scriberig/model";
import type { AgentSelection, BranchId, OperationId, SessionId } from "@scriberig/protocol";
import { useEffect, useMemo, useState, useSyncExternalStore, type ReactElement } from "react";
import { capabilityAvailable } from "../runtime";

const SESSION_STORAGE_KEY = "scriberig.agent.session";
const OPERATION_STORAGE_KEY = "scriberig.agent.operation";

/** Creates one browser-owned session identity for the view lifetime.
 * @returns A transport-safe session identifier.
 */
function createSessionId(): SessionId {
  try {
    const stored = globalThis.sessionStorage?.getItem(SESSION_STORAGE_KEY);
    if (stored) return stored as SessionId;
  } catch {
    // Browser storage may be unavailable; the generated identity remains valid.
  }
  const generated = globalThis.crypto?.randomUUID?.();
  const sessionId = (generated ?? `session-${Date.now().toString(36)}`) as SessionId;
  try {
    globalThis.sessionStorage?.setItem(SESSION_STORAGE_KEY, sessionId);
  } catch {
    // Browser storage may be unavailable; retain the in-memory identity.
  }
  return sessionId;
}

/** Converts the shared semantic selection into the agent request shape.
 * @param selection Current semantic selection.
 * @returns Provider-neutral agent selection, when one exists.
 */
function toAgentSelection(
  selection: ReturnType<typeof useSelection>["selection"],
): AgentSelection | undefined {
  if (selection === undefined) return undefined;
  return {
    entityId: selection.entityId,
    snapshotId: selection.snapshotId,
    ...(selection.revisionId === undefined ? {} : { revisionId: selection.revisionId }),
    ...(selection.projection?.location === undefined
      ? {}
      : { location: selection.projection.location }),
  };
}

/** Renders structured semantic scope and branch-aware agent controls.
 * @param props View properties.
 * @param props.branches Optional host-provided durable branch descriptors.
 * @returns The rendered session and agent view.
 */
export function SessionAgentView({
  branches = [],
  onReviewCandidate,
}: {
  branches?: readonly BranchDescriptor[];
  onReviewCandidate?: (migrationId: string) => void;
}): ReactElement {
  const { client, model, capabilities } = useScriberig();
  const { selection, setSelection } = useSelection();
  const agentState = useSyncExternalStore(
    (listener) => model.subscribeAgent(() => listener()),
    model.getAgentState.bind(model),
    model.getAgentState.bind(model),
  );
  const [sessionId] = useState(createSessionId);
  const [activeOperationId, setActiveOperationId] = useState<OperationId>();
  const [activeBranchId, setActiveBranchId] = useState<BranchId>();
  const [prompt, setPrompt] = useState("");
  const [status, setStatus] = useState("Ready");
  const { operation } = useOperation(activeOperationId);
  const visibleBranches = useMemo(() => {
    const discovered = Object.values(agentState.branches);
    return discovered.length > 0 ? discovered : branches;
  }, [agentState.branches, branches]);
  const stream = activeOperationId ? agentState.transientStreams[activeOperationId] : undefined;
  const activeActivity = activeBranchId ? agentState.activity[activeBranchId] : undefined;
  const currentSelection = toAgentSelection(selection);
  const canStartAgent = capabilityAvailable(capabilities, "agent_start");
  const canCancelOperation = capabilityAvailable(capabilities, "operation_cancel");
  useEffect(() => {
    let storedOperation: string | null = null;
    try {
      storedOperation = globalThis.sessionStorage?.getItem(OPERATION_STORAGE_KEY) ?? null;
    } catch {
      storedOperation = null;
    }
    const restore = async (): Promise<void> => {
      if (storedOperation !== null) {
        try {
          const restored = await client.getOperation(storedOperation);
          setActiveOperationId(restored.operationId);
          model.operations.set(restored);
          model.setAgentOperation(restored);
          if (restored.sessionId !== undefined) {
            model.setAgentSession(await client.getSession(restored.sessionId));
            return;
          }
        } catch {
          // The operation may have expired; the durable session remains useful.
        }
      }
      try {
        model.setAgentSession(await client.getSession(sessionId));
      } catch {
        // A newly-created session has no server record until the first submit.
      }
    };
    void restore();
  }, [client, model, sessionId]);
  const navigateResult = (handle: string): void => {
    if (selection === undefined) {
      setStatus(`Result ${handle} is ready to open`);
      return;
    }
    setSelection({ ...selection, entityId: handle as never });
  };

  const submit = async (prompt: string): Promise<void> => {
    if (!canStartAgent) {
      setStatus("Agent start is unavailable");
      return;
    }
    setStatus("Submitting");
    try {
      const accepted =
        activeBranchId === undefined
          ? await client.startAgent({
              sessionId,
              prompt,
              ...(currentSelection === undefined ? {} : { selection: currentSelection }),
            })
          : await client.continueAgent({
              sessionId,
              branchId: activeBranchId,
              prompt,
              ...(currentSelection === undefined ? {} : { selection: currentSelection }),
            });
      setActiveOperationId(accepted.operationId);
      try {
        globalThis.sessionStorage?.setItem(OPERATION_STORAGE_KEY, accepted.operationId);
      } catch {
        // Browser storage may be unavailable; operation state remains in memory.
      }
      if (accepted.branchId !== undefined) setActiveBranchId(accepted.branchId);
      model.operations.set({
        operationId: accepted.operationId,
        kind: "agent",
        status: "queued",
        cancellable: true,
        sessionId: accepted.sessionId,
        ...(accepted.branchId === undefined ? {} : { branchId: accepted.branchId }),
      });
      model.setAgentOperation({
        operationId: accepted.operationId,
        kind: "agent",
        status: "queued",
        cancellable: true,
        sessionId: accepted.sessionId,
        ...(accepted.branchId === undefined ? {} : { branchId: accepted.branchId }),
      });
      void client.getSession(accepted.sessionId).then((session) => model.setAgentSession(session));
      setStatus("Queued");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Agent request failed");
    }
  };

  const cancel = async (): Promise<void> => {
    if (activeOperationId === undefined) return;
    try {
      const cancelled = await client.cancelOperation(activeOperationId);
      model.operations.set(cancelled);
      model.setAgentOperation(cancelled);
      try {
        globalThis.sessionStorage?.removeItem(OPERATION_STORAGE_KEY);
      } catch {
        // Browser storage may be unavailable; terminal state is still authoritative.
      }
      setStatus("Cancelled");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Cancellation failed");
    }
  };

  const panelOperation: Operation | undefined = operation;
  return (
    <section aria-label="Session and Agent">
      <h1>Session / Agent</h1>
      <SessionBranchTree
        branches={visibleBranches}
        onSelect={(branch) => setActiveBranchId(branch.branchId)}
        onNavigate={navigateResult}
      />
      {activeActivity ? (
        <BranchActivityView page={activeActivity} onNavigate={navigateResult} />
      ) : null}
      <AgentComposer
        {...(currentSelection === undefined ? {} : { selection: currentSelection })}
        value={prompt}
        onChange={setPrompt}
        onSubmit={(prompt) => void submit(prompt)}
        disabled={!canStartAgent}
      />
      {panelOperation ? (
        <AgentOperationPanel
          operation={panelOperation}
          {...(stream === undefined ? {} : { stream })}
          {...(canCancelOperation ? { onCancel: () => void cancel() } : {})}
          {...(onReviewCandidate === undefined ? {} : { onReviewCandidate })}
        />
      ) : (
        <p role="status">{status}</p>
      )}
    </section>
  );
}
