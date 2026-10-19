/** @file Projects durable session activity into reusable client views. */
import type {
  BranchActivityPage,
  BranchChildrenPage,
  SessionId,
  SessionReadResponse,
} from "@scriberig/protocol";

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
  const recorded = branch.activity ?? [];
  if (recorded.length > 0) {
    return {
      branchId: branch.id,
      ...(recorded[0]?.session === undefined
        ? {}
        : { sessionId: recorded[0].session as SessionId }),
      items: recorded.map((item) => ({
        id: item.id,
        ordinal: item.sequence,
        kind: activityKind(item.kind),
        activityKind: item.kind,
        state: item.state,
        summary: item.summary,
        handles:
          item.kind === "BranchStarted"
            ? [item.id]
            : item.durableRef === undefined
              ? []
              : [item.durableRef],
        ...(item.parent === undefined ? {} : { parentId: item.parent }),
        ...(item.model === undefined ? {} : { model: item.model }),
        ...(item.thinkingLevel === undefined ? {} : { thinkingLevel: item.thinkingLevel }),
        ...(item.toolName === undefined ? {} : { toolName: item.toolName }),
        ...(item.toolCategory === undefined ? {} : { toolCategory: item.toolCategory }),
        ...(item.toolArguments === undefined ? {} : { toolArguments: item.toolArguments }),
        ...(item.toolResultPreview === undefined
          ? {}
          : { toolResultPreview: item.toolResultPreview }),
      })),
    };
  }
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

/** Converts a protocol activity kind to its presentation category.
 * @param kind Durable protocol activity kind.
 * @returns Compact category used by the activity view.
 */
function activityKind(
  kind: NonNullable<SessionReadResponse["branches"][number]["activity"]>[number]["kind"],
): BranchActivityPage["items"][number]["kind"] {
  switch (kind) {
    case "HumanInput":
      return "human";
    case "ModelInvocation":
    case "AssistantOutput":
    case "ReasoningSummary":
    case "RuntimeStatus":
    case "ModelRequest":
      return "model";
    case "ToolInvocation":
    case "ToolResult":
      return "tool";
    case "BranchStarted":
      return "child";
    case "BranchCompleted":
      return "result";
    case "OperationProgress":
    case "Validation":
    case "ModelSelectionChanged":
    case "ThinkingLevelChanged":
    case "Error":
      return "model";
    case "PlanCreated":
    case "PlanRevised":
    case "PlanNodeReady":
    case "PlanNodeStarted":
    case "PlanNodeCompleted":
    case "PlanNodeFailed":
    case "CoordinatorChanged":
    case "DependencyStateChanged":
      return "child";
    case "AssignmentChanged":
      return "model";
    case "SemanticOperation":
    case "ExternalProcess":
      return "tool";
  }
}
