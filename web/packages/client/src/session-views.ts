/** @file Normalizes and projects durable session reads for browser clients. */
import type {
  BranchActivityPage,
  BranchChildrenPage,
  BranchId,
  SessionId,
  SessionReadResponse,
} from "@scriberig/protocol";

/** Normalizes a wire session response. @param value Unknown response. @returns Public session tree. @throws For invalid responses. */
export function normalizeSession(value: unknown): SessionReadResponse {
  if (typeof value !== "object" || value === null)
    throw new Error("server returned an invalid session");
  const record = value as Record<string, unknown>;
  const session = record.session as Record<string, unknown> | undefined;
  if (typeof session?.id !== "string") throw new Error("server returned an incomplete session");
  return {
    session: { id: session.id as SessionId },
    branches: (Array.isArray(record.branches) ? record.branches : []).map(normalizeBranch),
  };
}

/** Normalizes a wire branch. @param value Unknown branch. @returns Public branch. */
function normalizeBranch(value: unknown): SessionReadResponse["branches"][number] {
  const branch = value as Record<string, unknown>;
  const result = branch.result as Record<string, unknown> | undefined;
  const parent = typeof branch.parent === "string" ? (branch.parent as BranchId) : undefined;
  const normalizedResult =
    result === undefined
      ? undefined
      : {
          id: String(result.id),
          references: Array.isArray(result.references)
            ? result.references.filter(
                (reference): reference is string => typeof reference === "string",
              )
            : [],
        };
  return {
    id: String(branch.id) as BranchId,
    ...(parent === undefined ? {} : { parent }),
    children: Array.isArray(branch.children)
      ? branch.children
          .filter((child): child is string => typeof child === "string")
          .map((child) => child as BranchId)
      : [],
    title: typeof branch.title === "string" ? branch.title : "",
    status: normalizeStatus(branch.status),
    turns: (Array.isArray(branch.turns) ? branch.turns : []).map(normalizeTurn),
    ...(normalizedResult === undefined ? {} : { result: normalizedResult }),
  };
}

/** Normalizes a branch status. @param value Unknown status. @returns Public status. */
function normalizeStatus(value: unknown): "Active" | "Completed" | "Abandoned" {
  const status = typeof value === "string" ? value.toLowerCase() : "active";
  return status === "completed" ? "Completed" : status === "abandoned" ? "Abandoned" : "Active";
}

/** Normalizes a wire turn. @param value Unknown turn. @returns Public turn. */
function normalizeTurn(value: unknown): SessionReadResponse["branches"][number]["turns"][number] {
  const turn = value as Record<string, unknown>;
  const agentOutput = turn.agentOutput ?? turn.agent_output;
  const toolInvocation = turn.toolInvocation ?? turn.tool_invocation;
  const toolResult = turn.toolResult ?? turn.tool_result;
  return {
    id: String(turn.id),
    ordinal: typeof turn.ordinal === "number" ? turn.ordinal : 0,
    hasHumanInput: Boolean(turn.hasHumanInput ?? turn.has_human_input),
    hasAgentOutput: Boolean(turn.hasAgentOutput ?? turn.has_agent_output),
    ...(typeof agentOutput === "string" ? { agentOutput } : {}),
    ...(typeof toolInvocation === "string" ? { toolInvocation } : {}),
    ...(typeof toolResult === "string" ? { toolResult } : {}),
  };
}

/** Projects a branch descriptor. @param branch Durable branch. @returns Compact descriptor. */
export function branchDescriptor(
  branch: SessionReadResponse["branches"][number],
): BranchChildrenPage["items"][number] {
  return {
    branchId: branch.id,
    name: branch.title,
    status: branch.status.toLowerCase() as BranchChildrenPage["items"][number]["status"],
    ...(branch.parent === undefined ? {} : { parentBranchId: branch.parent }),
    childCount: branch.children.length,
    activityCount: branch.turns.length,
    ...(branch.result === undefined
      ? {}
      : {
          result: {
            branchId: branch.id,
            resultId: branch.result.id,
            handles: branch.result.references,
          },
        }),
  };
}

/** Projects bounded branch activity. @param branch Durable branch. @returns Activity page. */
export function branchActivity(
  branch: SessionReadResponse["branches"][number],
): BranchActivityPage {
  return {
    branchId: branch.id,
    items: branch.turns.map((turn) => ({
      id: turn.id,
      ordinal: turn.ordinal,
      kind: turn.hasAgentOutput ? "model" : turn.toolInvocation ? "tool" : "human",
      summary: turn.agentOutput ?? (turn.hasHumanInput ? "Human input" : "Activity"),
      handles: [
        ...(turn.toolInvocation === undefined ? [] : [turn.toolInvocation]),
        ...(turn.toolResult === undefined ? [] : [turn.toolResult]),
      ],
    })),
  };
}
