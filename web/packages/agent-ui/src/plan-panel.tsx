/** @file Presents durable plan state and shared plan operations. */

import type {
  SessionId,
  SessionPlan,
  SessionPlanAttempt,
  SessionPlanEvidenceAssociation,
  SessionPlanGate,
} from "@scriberig/protocol";
import { useScriberig } from "@scriberig/react";
import { useEffect, useState, type ReactElement } from "react";

/** Displays plan nodes, exact-snapshot evidence, attempts, and recovery actions.
 * @param props Session and authoritative plan projection.
 * @param props.sessionId Owning durable session.
 * @param props.plans Current shared plan projections.
 * @param props.onPlanUpdated Refresh callback after a successful mutation.
 * @returns The plan inspection and action panel.
 */
export function PlanPanel({
  sessionId,
  plans,
  onPlanUpdated,
}: {
  sessionId: SessionId;
  plans: readonly SessionPlan[];
  onPlanUpdated: () => Promise<void>;
}): ReactElement | null {
  const { client } = useScriberig();
  const [selected, setSelected] = useState<{ plan: string; node: string }>();
  const [details, setDetails] = useState<{
    gates: readonly SessionPlanGate[];
    evidence: readonly SessionPlanEvidenceAssociation[];
    attempts: readonly SessionPlanAttempt[];
  }>();
  const [reason, setReason] = useState("");
  const [evidenceEntity, setEvidenceEntity] = useState("");
  const [evidenceNote, setEvidenceNote] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const plan = plans.find((item) => item.id === selected?.plan);
  const node = plan?.nodes.find((item) => item.id === selected?.node);

  useEffect(() => {
    let active = true;
    if (!plan || !node || !plan.snapshot) {
      setDetails(undefined);
      return () => {
        active = false;
      };
    }
    const request = {
      session: { id: sessionId },
      plan: plan.id,
      node: node.id,
      snapshot: plan.snapshot,
      offset: 0,
      limit: 64,
    };
    void Promise.all([client.getPlanNodeDetails(request), client.getPlanAttemptHistory(request)])
      .then(([page, history]) => {
        if (active)
          setDetails({ gates: page.gates, evidence: page.evidence, attempts: history.attempts });
      })
      .catch((error: unknown) => {
        if (active) setMessage(error instanceof Error ? error.message : "Plan details unavailable");
      });
    return () => {
      active = false;
    };
  }, [client, node, plan, sessionId]);

  if (plans.length === 0) return null;

  const mutate = async (action: () => Promise<unknown>): Promise<void> => {
    setBusy(true);
    setMessage("");
    try {
      await action();
      await onPlanUpdated();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Plan operation failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-label="Plans">
      <h2>Plans</h2>
      {plans.map((item) => (
        <section key={item.id} aria-label={`Plan ${item.goal}`}>
          <h3>
            {item.goal} ({item.status})
          </h3>
          <ul>
            {item.nodes.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  onClick={() => setSelected({ plan: item.id, node: entry.id })}
                >
                  {entry.title}: {entry.status} / {entry.effectiveStatus ?? entry.status}
                </button>
                {entry.dependencyBlockers.length ? (
                  <span> Blocked by {entry.dependencyBlockers.join(", ")}</span>
                ) : null}
                {entry.canRetry ? (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void mutate(() =>
                        client.retryPlanNode(sessionId, item.id, entry.id, item.revision),
                      )
                    }
                  >
                    Retry
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        </section>
      ))}
      {node && plan ? (
        <section aria-label={`Node details ${node.title}`}>
          <h3>{node.title}</h3>
          <p>{node.purpose}</p>
          <p>{node.summaryPreview ?? "No summary"}</p>
          <h4>Gates</h4>
          <label>
            Waiver reason{" "}
            <input value={reason} onChange={(event) => setReason(event.target.value)} />
          </label>
          <ul>
            {(details?.gates ?? []).map((gate) => {
              return (
                <li key={gate.id}>
                  {gate.phase} gate {gate.id}
                  {gate.waiver ? ` (waived: ${gate.waiver.reason})` : null}
                  {!gate.waiver ? (
                    <button
                      type="button"
                      disabled={busy || reason.trim().length === 0}
                      onClick={() =>
                        void mutate(() =>
                          client.mutatePlanSemantics(sessionId, plan.id, plan.revision, {
                            WaiveGate: { gate: gate.id, reason },
                          }),
                        )
                      }
                    >
                      Waive
                    </button>
                  ) : null}
                </li>
              );
            })}
          </ul>
          <h4>Evidence</h4>
          <label>
            Evidence entity{" "}
            <input
              value={evidenceEntity}
              onChange={(event) => setEvidenceEntity(event.target.value)}
            />
          </label>
          <label>
            Evidence note{" "}
            <input value={evidenceNote} onChange={(event) => setEvidenceNote(event.target.value)} />
          </label>
          <button
            type="button"
            disabled={busy || !evidenceEntity.trim()}
            onClick={() =>
              void mutate(() =>
                client.mutatePlanSemantics(sessionId, plan.id, plan.revision, {
                  AttachEvidence: {
                    owner: { Node: node.id },
                    role: "Informational",
                    evidence: {
                      kind: "Artifact",
                      reference: { entity: evidenceEntity.trim() },
                      attributes: {},
                    },
                    ...(evidenceNote.trim() ? { note: evidenceNote.trim() } : {}),
                  },
                }),
              )
            }
          >
            Attach evidence
          </button>
          <ul>
            {(details?.evidence ?? []).map((item) => {
              return (
                <li key={item.id}>
                  {String(item.evidence.kind)}: {item.evidence.reference.entity} ({item.role}){" "}
                  {item.note ?? ""}
                </li>
              );
            })}
          </ul>
          <h4>Attempts</h4>
          <ul>
            {(details?.attempts ?? []).map((attempt) => {
              return (
                <li key={attempt.id}>
                  {attempt.status} on {attempt.branch}
                </li>
              );
            })}
          </ul>
        </section>
      ) : null}
      {message ? <p role="status">{message}</p> : null}
    </section>
  );
}
