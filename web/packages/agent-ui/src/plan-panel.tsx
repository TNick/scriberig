/** @file Renders shared plan state and routes actions through the app client. */

import { useState, type ReactElement } from "react";
import type { BranchDescriptor } from "@scriberig/model";
import type { SessionPlan, SessionPlanMutation } from "@scriberig/protocol";

/** Renders plan gates, result evidence, and recoverable node actions. */
export function SessionPlanPanel({
  plans,
  branches,
  busy = false,
  onMutation,
}: {
  plans: readonly SessionPlan[];
  branches: readonly BranchDescriptor[];
  busy?: boolean;
  onMutation: (plan: SessionPlan, mutation: SessionPlanMutation) => Promise<void>;
}): ReactElement {
  const [waiverReasons, setWaiverReasons] = useState<Readonly<Record<string, string>>>({});
  const results = branches.flatMap((branch) =>
    branch.result === undefined ? [] : [{ branch, result: branch.result }],
  );

  return (
    <section aria-label="Session plans">
      <h2>Plans</h2>
      {plans.length === 0 ? <p>No durable plans.</p> : null}
      {plans.map((plan) => (
        <article key={plan.id} aria-label={`Plan ${plan.goal}`}>
          <h3>{plan.goal}</h3>
          <p>Plan status: {plan.status}</p>
          <ol aria-label={`Plan nodes for ${plan.goal}`}>
            {plan.nodes.map((node) => {
              const nodeGates = plan.gates.filter((gate) => gate.owner === node.id);
              const hasEntryResultGate = nodeGates.some(
                (gate) =>
                  gate.phase === "Entry" &&
                  "EvidencePresent" in gate.requirement &&
                  gate.requirement.EvidencePresent.kind === "BranchResult",
              );
              return (
                <li key={node.id}>
                  <h4>{node.title}</h4>
                  <p>
                    {node.status}
                    {node.effectiveStatus === undefined
                      ? ""
                      : ` · effective ${node.effectiveStatus}`}
                  </p>
                  {node.canRetry ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void onMutation(plan, { RetryFailedNode: { node: node.id } })}
                    >
                      Retry failed node
                    </button>
                  ) : null}
                  {!hasEntryResultGate && node.status !== "Running" ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() =>
                        void onMutation(plan, {
                          AddGate: {
                            owner: node.id,
                            phase: "Entry",
                            requirement: { EvidencePresent: { kind: "BranchResult" } },
                          },
                        })
                      }
                    >
                      Require branch-result evidence
                    </button>
                  ) : null}
                  {nodeGates.map((gate) => (
                    <div key={gate.id}>
                      <p>
                        {gate.phase} gate {gate.id}
                        {gate.waiver === undefined ? "" : ` · waived: ${gate.waiver.reason}`}
                      </p>
                      {gate.waiver === undefined ? (
                        <form
                          aria-label={`Waive gate ${gate.id}`}
                          onSubmit={(event) => {
                            event.preventDefault();
                            const reason = waiverReasons[gate.id]?.trim();
                            if (!reason) return;
                            void onMutation(plan, {
                              WaiveGate: { gate: gate.id, reason },
                            });
                          }}
                        >
                          <label>
                            Waiver reason
                            <input
                              value={waiverReasons[gate.id] ?? ""}
                              disabled={busy}
                              onChange={(event) =>
                                setWaiverReasons((current) => ({
                                  ...current,
                                  [gate.id]: event.currentTarget.value,
                                }))
                              }
                            />
                          </label>
                          <button
                            type="submit"
                            disabled={busy || !(waiverReasons[gate.id] ?? "").trim()}
                          >
                            Waive gate
                          </button>
                        </form>
                      ) : null}
                    </div>
                  ))}
                  {results.map(({ branch, result }) => {
                    const attached = plan.evidence.some(
                      (item) =>
                        "Node" in item.owner &&
                        item.owner.Node === node.id &&
                        item.evidence.kind === "BranchResult" &&
                        item.evidence.reference.entity === result.resultId,
                    );
                    return attached ? null : (
                      <button
                        key={`${node.id}:${result.resultId}`}
                        type="button"
                        disabled={busy}
                        onClick={() =>
                          void onMutation(plan, {
                            AttachEvidence: {
                              owner: { Node: node.id },
                              role: "Supports",
                              evidence: {
                                kind: "BranchResult",
                                reference: { entity: result.resultId },
                                attributes: { branch: branch.branchId },
                              },
                              ...(result.summary === undefined ? {} : { note: result.summary }),
                            },
                          })
                        }
                      >
                        Attach result from {branch.name}
                      </button>
                    );
                  })}
                </li>
              );
            })}
          </ol>
        </article>
      ))}
    </section>
  );
}
