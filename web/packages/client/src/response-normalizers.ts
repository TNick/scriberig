/** @file Normalizes responses from legacy and current semantic transports. */
import type {
  AgentAcceptedResponse,
  ModelRequestDetailResponse,
  Operation,
  SnapshotId,
} from "@scriberig/protocol";

/** Normalizes legacy and current operation response shapes.
 *
 * @param value Untrusted operation response from a transport.
 * @returns A normalized public operation.
 * @throws When the response is not an operation object.
 */
export function normalizeOperation(value: unknown): Operation {
  if (typeof value !== "object" || value === null) {
    throw new Error("server returned an invalid operation");
  }
  const record = value as Record<string, unknown>;
  if (typeof record.operationId === "string") return record as unknown as Operation;
  const reference = record.reference as Record<string, unknown> | undefined;
  const progress = record.progress as Record<string, unknown> | undefined;
  const result = record.result_snapshot as Record<string, unknown> | undefined;
  const status = record.status;
  const normalized: Operation = {
    operationId: String(reference?.id ?? "") as Operation["operationId"],
    kind: typeof record.kind === "string" ? record.kind.toLowerCase() : "unknown",
    status: (typeof status === "string" ? status.toLowerCase() : "failed") as Operation["status"],
    cancellable:
      typeof record.cancellable === "boolean"
        ? record.cancellable
        : typeof record.can_cancel === "boolean"
          ? record.can_cancel
          : status === "Queued" ||
            status === "Running" ||
            status === "queued" ||
            status === "running",
  };
  if (typeof progress?.percent === "number") normalized.progress = progress.percent / 100;
  if (typeof result?.id === "string") normalized.resultSnapshotId = result.id as SnapshotId;
  if (typeof record.error === "string") normalized.error = record.error;
  const agent = record.agent_result as Record<string, unknown> | undefined;
  if (agent !== undefined) {
    const session = agent.session as Record<string, unknown> | undefined;
    const branch = agent.root_branch as Record<string, unknown> | undefined;
    const produced = Array.isArray(agent.produced_changes) ? agent.produced_changes : [];
    const producedChanges = produced.flatMap((item) =>
      isObject(item) && typeof item.id === "string" ? [item.id as never] : [],
    );
    const validationResults = normalizeStrings(agent.validation_results ?? agent.validationResults);
    const unresolvedQuestions = normalizeStrings(
      agent.unresolved_questions ?? agent.unresolvedQuestions,
    );
    const candidateChanges = Array.isArray(agent.candidate_changes)
      ? agent.candidate_changes
          .filter(
            (item): item is Record<string, unknown> => typeof item === "object" && item !== null,
          )
          .map((item) => ({
            ...(typeof item.migration === "string" ? { migrationId: item.migration as never } : {}),
            ...((item.entity as Record<string, unknown> | undefined)?.id
              ? { entityId: (item.entity as Record<string, unknown>).id as never }
              : {}),
            ...((item.candidate_snapshot as Record<string, unknown> | undefined)?.id
              ? {
                  candidateSnapshotId: (item.candidate_snapshot as Record<string, unknown>)
                    .id as never,
                }
              : {}),
            validationRefs: Array.isArray(item.validation)
              ? item.validation.filter((entry): entry is string => typeof entry === "string")
              : [],
          }))
      : [];
    normalized.result = {
      sessionId: String(session?.id ?? "") as Operation["sessionId"] & string,
      rootBranchId: String(branch?.id ?? "") as Operation["branchId"] & string,
      branchResultId: String(agent.branch_result ?? ""),
      finalOutput: typeof agent.final_output === "string" ? agent.final_output : "",
      producedChanges,
      validationResults,
      unresolvedQuestions,
      handles: produced.flatMap((item) =>
        typeof item === "object" && item !== null && typeof item.id === "string" ? [item.id] : [],
      ),
      candidateChanges,
    };
    normalized.sessionId = normalized.result.sessionId;
    normalized.branchId = normalized.result.rootBranchId;
  }
  if (normalized.status === "failed" && normalized.error !== undefined) {
    normalized.failure = { category: "operation_failure", message: normalized.error };
  }
  return normalized;
}

/** Narrows a JSON object from an untrusted protocol response.
 * @param value Candidate response value.
 * @returns Whether the value is a non-array object.
 */
function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Normalizes a list of safe wire strings.
 * @param value Unknown wire value.
 * @returns String items from an array, or an empty array.
 */
function normalizeStrings(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

/** Normalizes an accepted agent response from either wire generation.
 * @param value Unknown accepted response from a transport.
 * @returns Provider-neutral accepted handles.
 * @throws When the response does not contain an operation identity.
 */
export function normalizeAcceptedAgent(value: unknown): AgentAcceptedResponse {
  if (typeof value !== "object" || value === null)
    throw new Error("server returned an invalid agent acceptance");
  const record = value as Record<string, unknown>;
  const operation = record.operation as Record<string, unknown> | undefined;
  const reference = operation?.reference as Record<string, unknown> | undefined;
  const operationId = record.operationId ?? reference?.id;
  const session = record.session as Record<string, unknown> | undefined;
  const branch = record.branch as Record<string, unknown> | undefined;
  if (typeof operationId !== "string" || typeof (session?.id ?? record.sessionId) !== "string") {
    throw new Error("server returned incomplete agent handles");
  }
  const accepted = {
    operationId: operationId as AgentAcceptedResponse["operationId"],
    sessionId: String(session?.id ?? record.sessionId) as AgentAcceptedResponse["sessionId"],
  };
  const branchId =
    typeof branch?.id === "string"
      ? branch.id
      : typeof record.branchId === "string"
        ? record.branchId
        : undefined;
  return branchId === undefined
    ? accepted
    : { ...accepted, branchId: branchId as NonNullable<AgentAcceptedResponse["branchId"]> };
}

/** Normalizes a captured request detail response across Rust and browser casing.
 * @param value Unknown request detail from the transport.
 * @returns The safe request detail with browser field casing.
 * @throws When required request identity fields are absent.
 */
export function normalizeModelRequestDetail(value: unknown): ModelRequestDetailResponse {
  if (!isObject(value)) throw new Error("server returned invalid model request detail");
  const invocation = value.invocation;
  const model = value.model;
  if (typeof invocation !== "string" || typeof model !== "string") {
    throw new Error("server returned incomplete model request detail");
  }
  const providerId = value.provider_id ?? value.providerId;
  const thinkingLevel = value.thinking_level ?? value.thinkingLevel;
  const profileValue = value.profile;
  const profile =
    isObject(profileValue) &&
    typeof profileValue.identity === "string" &&
    typeof profileValue.revision === "string"
      ? { identity: profileValue.identity, revision: profileValue.revision }
      : undefined;
  const activeComponentsValue = value.active_components ?? value.activeComponents;
  const activeComponents = Array.isArray(activeComponentsValue)
    ? activeComponentsValue.filter(
        (reference): reference is { identity: string; revision: string } =>
          isObject(reference) &&
          typeof reference.identity === "string" &&
          typeof reference.revision === "string",
      )
    : undefined;
  const appliedSettings = value.applied_settings ?? value.appliedSettings;
  const appliedSettingsFingerprint =
    value.applied_settings_fingerprint ?? value.appliedSettingsFingerprint;
  return {
    invocation,
    model,
    ...(typeof providerId === "string" ? { providerId } : {}),
    ...(typeof thinkingLevel === "string" ? { thinkingLevel } : {}),
    ...(profile ? { profile } : {}),
    ...(activeComponents ? { activeComponents } : {}),
    ...(appliedSettings !== undefined ? { appliedSettings } : {}),
    ...(typeof appliedSettingsFingerprint === "string" ? { appliedSettingsFingerprint } : {}),
    messages: Array.isArray(value.messages)
      ? (value.messages as ModelRequestDetailResponse["messages"])
      : [],
    tools: Array.isArray(value.tools) ? (value.tools as ModelRequestDetailResponse["tools"]) : [],
    truncated: value.truncated === true,
  };
}
