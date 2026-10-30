/** @file Composes the reusable branch-aware agent workspace view. */

import {
  AgentComposer,
  AgentModelControls,
  AgentPresentationControls,
  AgentOperationPanel,
  SessionPlanPanel,
  BranchActivityView,
  SessionBranchTree,
} from "@scriberig/agent-ui";
import { ProjectCandidateReview } from "@scriberig/components";
import { ProjectCandidateController } from "@scriberig/model";
import {
  useAgentExecutionSelection,
  useOperation,
  useSelection,
  useScriberig,
} from "@scriberig/react";
import { agentSelectionFromSemanticSelection } from "@scriberig/editor-core";
import type { BranchDescriptor, Operation } from "@scriberig/model";
import type {
  BranchId,
  EntityId,
  OperationId,
  SessionId,
  SessionPlan,
  SessionPlanMutation,
} from "@scriberig/protocol";
import { useEffect, useMemo, useState, useSyncExternalStore, type ReactElement } from "react";
import { capabilityAvailable, type AppRuntime, type ProjectCandidate } from "../runtime";
const OPERATION_STORAGE_KEY = "scriberig.agent.operation";
import { getOrCreateSessionId } from "../session-id";

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
  const runtime = useScriberig() as unknown as AppRuntime;
  const { selection, setSelection } = useSelection();
  const agentState = useSyncExternalStore(
    (listener) => model.subscribeAgent(() => listener()),
    model.getAgentState.bind(model),
    model.getAgentState.bind(model),
  );
  const [sessionId] = useState(getOrCreateSessionId);
  const [activeOperationId, setActiveOperationId] = useState<OperationId>();
  const [activeBranchId, setActiveBranchId] = useState<BranchId>();
  const [prompt, setPrompt] = useState("");
  const [status, setStatus] = useState("Ready");
  const [reviewCandidateId, setReviewCandidateId] = useState<string>();
  const [planMutationPending, setPlanMutationPending] = useState(false);
  const { modelLibrary, presentation } = agentState;
  const candidateController = useMemo(
    () =>
      reviewCandidateId === undefined
        ? undefined
        : new ProjectCandidateController(client, reviewCandidateId),
    [client, reviewCandidateId],
  );
  const executionSelection = useAgentExecutionSelection(sessionId);
  const { operation } = useOperation(activeOperationId);
  const visibleBranches = useMemo(() => {
    const discovered = Object.values(agentState.branches);
    return discovered.length > 0 ? discovered : branches;
  }, [agentState.branches, branches]);
  const stream = activeOperationId ? agentState.transientStreams[activeOperationId] : undefined;
  const activeActivity = activeBranchId ? agentState.activity[activeBranchId] : undefined;
  const currentSelection = agentSelectionFromSemanticSelection(
    selection,
    runtime.activeWorkspace?.project.id,
    runtime.activeWorkspace?.rootEntity as EntityId | undefined,
  );
  const canStartAgent = capabilityAvailable(capabilities, "agent_start");
  const canCancelOperation = capabilityAvailable(capabilities, "operation_cancel");
  useEffect(() => {
    let active = true;
    void client
      .getModelLibrary()
      .then((library) => {
        if (active) model.setAgentModelLibrary(library);
      })
      .catch((error: unknown) => {
        if (active) setStatus(error instanceof Error ? error.message : "Model library unavailable");
      });
    void client
      .getAgentPresentationPreferences()
      .then(({ preferences }) => {
        if (active) {
          model.setSharedAgentPresentation({
            reasoning: preferences.reasoning,
            toolInvocation: preferences.toolInvocation,
            toolOutput: preferences.toolOutput,
            prompt: preferences.prompt,
          });
        }
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [client, model]);
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
  useEffect(() => {
    if (!agentState.stale) return;
    let active = true;
    void client
      .getSession(sessionId)
      .then((session) => {
        if (active) model.setAgentSession(session);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [agentState.stale, client, model, sessionId]);
  const navigateResult = (handle: string): void => {
    if (selection === undefined) {
      setStatus(`Result ${handle} is ready to open`);
      return;
    }
    setSelection({ ...selection, entityId: handle as never });
  };

  const submit = async (prompt: string): Promise<void> => {
    if (!canStartAgent && runtime.activeWorkspace === undefined) {
      setStatus("Agent start is unavailable");
      return;
    }
    setStatus("Submitting");
    try {
      if (runtime.activeWorkspace !== undefined && currentSelection !== undefined) {
        const candidate = await client.project<ProjectCandidate & { scope: unknown }>("agent", {
          project: runtime.activeWorkspace.project.id,
          entity: currentSelection.entityId,
          snapshot: currentSelection.snapshotId,
          prompt,
        });
        runtime.setAgentCandidate(candidate);
        setStatus("Candidate ready for review");
        onReviewCandidate?.(candidate.candidate);
        return;
      }
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

  const selectModel = async (providerId: string, modelId: string): Promise<void> => {
    if (providerId.length === 0 || modelId.length === 0) return;
    setStatus("Updating model");
    try {
      await executionSelection.selectModel(providerId, modelId);
      setStatus("Model updated for the next invocation");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Model selection failed");
    }
  };

  const selectThinkingLevel = async (level: string): Promise<void> => {
    if (level.length === 0) return;
    setStatus("Updating thinking level");
    try {
      await executionSelection.selectThinkingLevel(level);
      setStatus("Thinking level updated for the next invocation");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Thinking selection failed");
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

  const mutatePlan = async (plan: SessionPlan, mutation: SessionPlanMutation): Promise<void> => {
    setPlanMutationPending(true);
    setStatus("Updating plan");
    try {
      const updated = await client.mutatePlan({
        session: { id: sessionId },
        plan: plan.id,
        expectedRevision: plan.revision,
        mutation,
      });
      model.setAgentPlan(updated);
      setStatus("Plan updated");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Plan update failed");
    } finally {
      setPlanMutationPending(false);
    }
  };

  const panelOperation: Operation | undefined = operation;
  return (
    <section aria-label="Session and Agent">
      <h1>Session / Agent</h1>
      {agentState.selectionActivities.slice(-50).map((activity, index) => (
        <output
          key={`${activity.selectionRevision ?? "event"}:${activity.sequence ?? index}`}
          aria-label="Agent model selection activity"
        >
          {activity.summary}
        </output>
      ))}
      <SessionBranchTree
        branches={visibleBranches}
        onSelect={(branch) => setActiveBranchId(branch.branchId)}
        onNavigate={navigateResult}
        onReviewCandidate={setReviewCandidateId}
      />
      <SessionPlanPanel
        plans={agentState.plans}
        branches={visibleBranches}
        busy={planMutationPending}
        onMutation={mutatePlan}
      />
      {modelLibrary ? (
        <AgentModelControls
          library={modelLibrary}
          selection={agentState.execution}
          onModelChange={(providerId, modelId) => void selectModel(providerId, modelId)}
          onThinkingChange={(level) => void selectThinkingLevel(level)}
          disabled={executionSelection.mutating || !canStartAgent}
        />
      ) : null}
      <AgentPresentationControls
        value={presentation}
        onChange={(value) => model.setAgentPresentation(value)}
      />
      {activeActivity ? (
        <BranchActivityView
          page={activeActivity}
          transientItems={agentState.transientActivities[activeActivity.branchId] ?? []}
          presentation={presentation}
          onNavigate={navigateResult}
          onReviewCandidate={setReviewCandidateId}
          onLoadModelRequest={(session, branch, invocation) =>
            client.getModelRequestDetail(session as SessionId, branch as BranchId, invocation)
          }
          onLoadToolResult={(session, branch, result) =>
            client.getToolResultDetail(session as SessionId, branch as BranchId, result)
          }
        />
      ) : null}
      {candidateController ? (
        <ProjectCandidateReview
          key={reviewCandidateId}
          controller={candidateController}
          onApplied={(result) => {
            const snapshotId = result.successor as never;
            model.apply({ kind: "snapshotAdvanced", snapshotId });
            if (selection !== undefined) {
              const updatedSelection = { ...selection, snapshotId };
              delete updatedSelection.revisionId;
              delete updatedSelection.anchor;
              setSelection(updatedSelection);
            }
            setReviewCandidateId(undefined);
          }}
        />
      ) : null}
      <AgentComposer
        {...(currentSelection === undefined ? {} : { selection: currentSelection })}
        value={prompt}
        onChange={setPrompt}
        onSubmit={(prompt) => void submit(prompt)}
        disabled={!canStartAgent && runtime.activeWorkspace === undefined}
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
