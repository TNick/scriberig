/** @file Creates plans and exposes revision-checked lifecycle actions. */

import type {
  BranchDescriptor,
  BranchId,
  SessionId,
  SessionPlan,
  SessionPlanNode,
} from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useState, type FormEvent, type ReactElement } from "react";

/** Props for creating an anchored plan from a session branch. */
interface PlanCreateFormProps {
  /** Owning durable session identity. */
  sessionId: SessionId;
  /** Available durable branch heads for exact plan anchors. */
  branches: readonly BranchDescriptor[];
  /** Refreshes the authoritative session after a successful write. */
  onPlanUpdated: () => Promise<void>;
}

/** Renders a minimal structured plan creation operation.
 * @param props Session identity, branch anchors, and refresh callback.
 * @returns The plan creation form or an unavailable-anchor status.
 */
export function PlanCreateForm({
  sessionId,
  branches,
  onPlanUpdated,
}: PlanCreateFormProps): ReactElement {
  const { client } = useScriberig();
  const [branch, setBranch] = useState<BranchId | "">("");
  const [goal, setGoal] = useState("");
  const [title, setTitle] = useState("");
  const [purpose, setPurpose] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!branch) return;
    setBusy(true);
    setMessage("");
    try {
      await client.createPlan({
        session: { id: sessionId },
        anchor: { BranchHead: { branch } },
        goal: goal.trim(),
        nodes: [
          {
            key: "initial",
            parent_index: null,
            title: title.trim(),
            purpose: purpose.trim(),
            expected_result: null,
            semantic_scope: [],
            assignment: null,
          },
        ],
        dependencies: [],
      });
      await onPlanUpdated();
      setGoal("");
      setTitle("");
      setPurpose("");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan creation failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <form aria-label="Create plan" onSubmit={(event) => void submit(event)}>
      <label>
        Anchor branch{" "}
        <select value={branch} onChange={(event) => setBranch(event.target.value as BranchId | "")}>
          <option value="">Select a branch</option>
          {branches
            .filter((item) => item.status !== "abandoned")
            .map((item) => (
              <option key={item.branchId} value={item.branchId}>
                {item.name}
              </option>
            ))}
        </select>
      </label>
      <label>
        Plan goal <input value={goal} onChange={(event) => setGoal(event.target.value)} />
      </label>
      <label>
        First step title <input value={title} onChange={(event) => setTitle(event.target.value)} />
      </label>
      <label>
        First step purpose{" "}
        <input value={purpose} onChange={(event) => setPurpose(event.target.value)} />
      </label>
      <button
        type="submit"
        disabled={busy || !branch || !goal.trim() || !title.trim() || !purpose.trim()}
      >
        Create plan
      </button>
      {message ? <p role="status">{message}</p> : null}
    </form>
  );
}

/** Props for a plan's allowed lifecycle operations. */
interface PlanLifecycleActionsProps {
  /** Owning durable session identity. */
  sessionId: SessionId;
  /** Current authoritative plan projection and revision. */
  plan: SessionPlan;
  /** Refreshes the authoritative session after a successful write. */
  onPlanUpdated: () => Promise<void>;
}

/** Props for a repair replan that replaces one failed node. */
interface PlanReplanFormProps {
  /** Owning durable session identity. */
  sessionId: SessionId;
  /** Current authoritative plan revision and history. */
  plan: SessionPlan;
  /** Failed node to supersede. */
  node: SessionPlanNode;
  /** Refreshes the authoritative session after a successful write. */
  onPlanUpdated: () => Promise<void>;
}

/** Renders an exact-revision failed-node replacement while retaining results.
 * @param props Session, plan, failed node, and refresh callback.
 * @returns The replan form when the plan permits repair, otherwise no control.
 */
export function PlanReplanForm({
  sessionId,
  plan,
  node,
  onPlanUpdated,
}: PlanReplanFormProps): ReactElement | null {
  const { client } = useScriberig();
  const [title, setTitle] = useState("");
  const [purpose, setPurpose] = useState("");
  const [reason, setReason] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  if (
    node.status !== "Failed" ||
    (plan.status !== "Failed" && plan.status !== "Paused") ||
    node.supersededBy !== undefined
  ) {
    return null;
  }

  const submit = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      await client.replan({
        session: { id: sessionId },
        plan: plan.id,
        expected_revision: plan.revision,
        additions: [
          {
            key: "repair",
            parent_index: null,
            title: title.trim(),
            purpose: purpose.trim(),
            expected_result: null,
            semantic_scope: [],
            assignment: null,
          },
        ],
        dependencies: [],
        removed_dependencies: [],
        superseded: [[node.id, "repair"]],
        assignment_changes: [],
        preserved_terminal_nodes: plan.nodes
          .filter((item) => item.status === "Completed")
          .map((item) => item.id),
        reason: reason.trim(),
      });
      await onPlanUpdated();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan repair failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <form aria-label={`Replace failed node ${node.title}`} onSubmit={(event) => void submit(event)}>
      <label>
        Replacement title <input value={title} onChange={(event) => setTitle(event.target.value)} />
      </label>
      <label>
        Replacement purpose{" "}
        <input value={purpose} onChange={(event) => setPurpose(event.target.value)} />
      </label>
      <label>
        Replan reason <input value={reason} onChange={(event) => setReason(event.target.value)} />
      </label>
      <button type="submit" disabled={busy || !title.trim() || !purpose.trim() || !reason.trim()}>
        Replace failed work
      </button>
      {message ? <p role="status">{message}</p> : null}
    </form>
  );
}

/** Renders lifecycle actions allowed by the current plan status.
 * @param props Session, plan projection, and refresh callback.
 * @returns The lifecycle action controls.
 */
export function PlanLifecycleActions({
  sessionId,
  plan,
  onPlanUpdated,
}: PlanLifecycleActionsProps): ReactElement | null {
  const { client } = useScriberig();
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const actions =
    plan.status === "Planned"
      ? (["Start", "Stop"] as const)
      : plan.status === "Running"
        ? (["Pause", "Stop", "Complete"] as const)
        : plan.status === "Paused"
          ? (["Resume", "Stop"] as const)
          : [];

  if (actions.length === 0) return null;
  const apply = async (action: (typeof actions)[number]): Promise<void> => {
    setBusy(true);
    setMessage("");
    try {
      await client.mutatePlanLifecycle(sessionId, plan.id, plan.revision, action);
      await onPlanUpdated();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan lifecycle operation failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div aria-label={`Lifecycle actions for ${plan.goal}`}>
      {actions.map((action) => (
        <button key={action} type="button" disabled={busy} onClick={() => void apply(action)}>
          {action}
        </button>
      ))}
      {message ? <p role="status">{message}</p> : null}
    </div>
  );
}
