/** @file Edits node summaries and creates typed plan gates. */

import type { SessionId, SessionPlan, SessionPlanNode } from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useEffect, useState, type FormEvent, type ReactElement } from "react";

/** Props for authoring summary and gate semantics on one plan node.
 *
 * Public members: sessionId, plan, node, and onPlanUpdated.
 * Private members: None.
 */
interface PlanNodeAuthoringFormProps {
  /** Owning durable session identity. */
  sessionId: SessionId;
  /** Current authoritative plan revision. */
  plan: SessionPlan;
  /** Node receiving the summary or gate. */
  node: SessionPlanNode;
  /** Refreshes the authoritative session after a successful write. */
  onPlanUpdated: () => Promise<void>;
}

/** Renders shared summary-update and typed-gate operations for a node.
 * @param props Session, plan, node, and refresh callback.
 * @returns The node authoring forms.
 */
export function PlanNodeAuthoringForm({
  sessionId,
  plan,
  node,
  onPlanUpdated,
}: PlanNodeAuthoringFormProps): ReactElement {
  const { client } = useScriberig();
  const [summary, setSummary] = useState(node.summaryPreview ?? "");
  const [phase, setPhase] = useState<"Entry" | "Completion">("Entry");
  const [requireValidation, setRequireValidation] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    setSummary(node.summaryPreview ?? "");
  }, [node.id, node.summaryPreview]);

  const mutate = async (
    event: FormEvent<HTMLFormElement>,
    operation: () => Promise<unknown>,
  ): Promise<void> => {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      await operation();
      await onPlanUpdated();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan operation failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <form
        aria-label={`Update summary for ${node.title}`}
        onSubmit={(event) =>
          void mutate(event, () =>
            client.mutatePlanSemantics(sessionId, plan.id, plan.revision, {
              UpdateSummary: { node: node.id, text: summary.trim(), sources: [] },
            }),
          )
        }
      >
        <label>
          Summary <textarea value={summary} onChange={(event) => setSummary(event.target.value)} />
        </label>
        <button type="submit" disabled={busy || !summary.trim()}>
          Save summary
        </button>
      </form>
      <form
        aria-label={`Add gate to ${node.title}`}
        onSubmit={(event) =>
          void mutate(event, () =>
            client.mutatePlanSemantics(sessionId, plan.id, plan.revision, {
              AddGate: {
                owner: node.id,
                phase,
                requirement: requireValidation
                  ? { ValidationPassed: {} }
                  : { EvidencePresent: { kind: "ValidationResult" } },
              },
            }),
          )
        }
      >
        <label>
          Gate phase{" "}
          <select value={phase} onChange={(event) => setPhase(event.target.value as typeof phase)}>
            <option value="Entry">Entry</option>
            <option value="Completion">Completion</option>
          </select>
        </label>
        <label>
          Gate requirement{" "}
          <select
            value={requireValidation ? "validation" : "evidence"}
            onChange={(event) => setRequireValidation(event.target.value === "validation")}
          >
            <option value="validation">Validation passed</option>
            <option value="evidence">Validation evidence present</option>
          </select>
        </label>
        <button type="submit" disabled={busy}>
          Add gate
        </button>
      </form>
      {message ? <p role="status">{message}</p> : null}
    </>
  );
}
