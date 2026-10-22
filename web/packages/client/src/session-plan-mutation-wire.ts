/** @file Converts typed plan mutations to protocol wire values. */
import type { SessionPlanMutation } from "@scriberig/protocol";

/** Converts typed client mutation input to the Rust protocol JSON casing.
 * @param mutation Typed semantic plan mutation.
 * @returns Externally tagged request payload using protocol field names.
 */
export function planMutationToWire(mutation: SessionPlanMutation): unknown {
  if (typeof mutation === "string") return mutation;
  if ("AddNextStep" in mutation) {
    return { AddNextStep: { node: nodeDraftToWire(mutation.AddNextStep.node) } };
  }
  if ("Decompose" in mutation) {
    return {
      Decompose: {
        parent: mutation.Decompose.parent,
        children: mutation.Decompose.children.map(nodeDraftToWire),
        required: mutation.Decompose.required,
      },
    };
  }
  if ("AddGate" in mutation) return { AddGate: mutation.AddGate };
  if ("WaiveGate" in mutation) return { WaiveGate: mutation.WaiveGate };
  if ("AttachEvidence" in mutation) return { AttachEvidence: mutation.AttachEvidence };
  if ("UpdateSummary" in mutation) return { UpdateSummary: mutation.UpdateSummary };
  if ("RetryFailedNode" in mutation) return { RetryFailedNode: mutation.RetryFailedNode };
  return "RecoverFailedPlan";
}

/** Converts one model assignment draft to the Rust protocol's snake-case form.
 * @param node Request-local proposed work node.
 * @returns Wire-safe node record.
 */
function nodeDraftToWire(node: import("@scriberig/protocol").SessionPlanNodeDraft): unknown {
  return {
    key: node.key,
    parent_index: node.parentIndex ?? null,
    title: node.title,
    purpose: node.purpose,
    expected_result:
      node.expectedResult === undefined
        ? null
        : {
            kind: node.expectedResult.kind,
            requires_handle: node.expectedResult.requiresHandle,
          },
    semantic_scope: node.semanticScope,
    assignment:
      node.assignment === undefined
        ? null
        : {
            provider_id: node.assignment.providerId,
            model_id: node.assignment.modelId,
            thinking_level: node.assignment.thinkingLevel ?? null,
          },
  };
}
