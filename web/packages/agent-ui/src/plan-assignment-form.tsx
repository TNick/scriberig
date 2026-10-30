/** @file Assigns plan nodes using the upstream model library. */

import type { SessionId, SessionPlan, SessionPlanNode } from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useEffect, useState, type FormEvent, type ReactElement } from "react";

/** Props for a snapshot-aware node assignment form.
 *
 * Public members: sessionId, plan, node, and onPlanUpdated.
 * Private members: None.
 */
interface PlanAssignmentFormProps {
  /** Owning durable session identity. */
  sessionId: SessionId;
  /** Current authoritative plan revision. */
  plan: SessionPlan;
  /** Node receiving the assignment. */
  node: SessionPlanNode;
  /** Refreshes the authoritative session after a successful write. */
  onPlanUpdated: () => Promise<void>;
}

/** Renders a model assignment selector from the upstream-derived model library.
 * @param props Session, plan, node, and refresh callback.
 * @returns The assignment form, or a status message when model data is unavailable.
 */
export function PlanAssignmentForm({
  sessionId,
  plan,
  node,
  onPlanUpdated,
}: PlanAssignmentFormProps): ReactElement {
  const { client } = useScriberig();
  const [library, setLibrary] = useState<Awaited<ReturnType<typeof client.getModelLibrary>>>();
  const [modelSelection, setModelSelection] = useState("");
  const [thinkingLevel, setThinkingLevel] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getModelLibrary()
      .then((response) => {
        if (active) setLibrary(response);
      })
      .catch((error: unknown) => {
        if (active)
          setMessage(error instanceof Error ? error.message : "Model library unavailable");
      });
    return () => {
      active = false;
    };
  }, [client]);

  useEffect(() => {
    const assignment = node.assignment;
    setModelSelection(
      assignment ? JSON.stringify([assignment.providerId, assignment.modelId]) : "",
    );
    setThinkingLevel(assignment?.thinkingLevel ?? "");
  }, [node.id, node.assignment]);

  const selectedModel = library?.models.find(
    (model) => JSON.stringify([model.providerId, model.modelId]) === modelSelection,
  );

  const save = async (event: FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      await client.setPlanAssignment({
        session: { id: sessionId },
        plan: plan.id,
        node: node.id,
        expected_revision: plan.revision,
        assignment:
          selectedModel === undefined
            ? null
            : {
                provider_id: selectedModel.providerId,
                model_id: selectedModel.modelId,
                ...(thinkingLevel ? { thinking_level: thinkingLevel } : {}),
              },
      });
      await onPlanUpdated();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan assignment failed");
    } finally {
      setBusy(false);
    }
  };

  if (!library) return <p role="status">{message || "Loading model library"}</p>;

  return (
    <>
      <form aria-label={`Assign model to ${node.title}`} onSubmit={(event) => void save(event)}>
        <label>
          Model assignment{" "}
          <select
            value={modelSelection}
            onChange={(event) => {
              setModelSelection(event.target.value);
              setThinkingLevel("");
            }}
          >
            <option value="">Inherit default</option>
            {library.models.map((model) => (
              <option
                key={JSON.stringify([model.providerId, model.modelId])}
                value={JSON.stringify([model.providerId, model.modelId])}
              >
                {model.displayName}
              </option>
            ))}
          </select>
        </label>
        {selectedModel && selectedModel.thinkingLevels.length > 0 ? (
          <label>
            Thinking level{" "}
            <select
              value={thinkingLevel}
              onChange={(event) => setThinkingLevel(event.target.value)}
            >
              <option value="">Use model default</option>
              {selectedModel.thinkingLevels.map((level) => (
                <option key={level} value={level}>
                  {level}
                </option>
              ))}
            </select>
          </label>
        ) : null}
        <button type="submit" disabled={busy}>
          Save assignment
        </button>
      </form>
      {message ? <p role="status">{message}</p> : null}
    </>
  );
}
