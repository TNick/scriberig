/** @file Normalizes responses from legacy and current semantic transports. */
import type {
  AgentAcceptedResponse,
  ModelRequestDetailResponse,
  ObjectDescription,
  Operation,
  SnapshotId,
} from "@scriberig/protocol";

/** Converts the pinned inspect response to the shared object-description model.
 * @param value Untrusted response from an inspect operation.
 * @returns The normalized semantic object description.
 * @throws When required entity or snapshot identities are absent.
 */
export function normalizeObjectDescription(value: unknown): ObjectDescription {
  if (!isObject(value)) throw new Error("server returned an invalid inspection");
  if (isObject(value.summary) && isObject(value.summary.ref)) {
    return value as unknown as ObjectDescription;
  }
  const entity = value.entity;
  const revision = value.revision;
  const snapshot = value.snapshot;
  if (
    !isObject(entity) ||
    typeof entity.id !== "string" ||
    !isObject(revision) ||
    typeof revision.id !== "string" ||
    !isObject(snapshot) ||
    typeof snapshot.id !== "string"
  ) {
    throw new Error("server returned an incomplete inspection");
  }
  const summary = typeof value.summary === "string" ? value.summary : undefined;
  return {
    summary: {
      ref: {
        entityId: entity.id as ObjectDescription["summary"]["ref"]["entityId"],
        snapshotId: snapshot.id as ObjectDescription["summary"]["ref"]["snapshotId"],
        ...(revision.id === undefined
          ? {}
          : {
              revisionId: revision.id as NonNullable<
                ObjectDescription["summary"]["ref"]["revisionId"]
              >,
            }),
      },
      typeId: (canonicalText(value.object_type) ??
        "unknown") as ObjectDescription["summary"]["typeId"],
      ...(summary === undefined ? {} : { label: summary }),
    },
    ...(summary === undefined ? {} : { text: summary }),
  };
}

/** Converts a pinned list response into a bounded browser collection page.
 * @param value Untrusted response from a list operation.
 * @returns The normalized collection page.
 * @throws When a result does not carry a typed semantic target.
 */
export function normalizeCollectionPage(value: unknown): {
  items: ObjectDescription["summary"][];
  nextCursor?: string;
} {
  if (!isObject(value)) throw new Error("server returned an invalid list response");
  if (Array.isArray(value.items))
    return value as unknown as ReturnType<typeof normalizeCollectionPage>;
  const envelope = isObject(value.result) ? value.result : undefined;
  const page = envelope !== undefined && isObject(envelope.page) ? envelope.page : undefined;
  if (page === undefined || !Array.isArray(page.items)) {
    throw new Error("server returned an incomplete list response");
  }
  const items = page.items.map((item) => {
    if (!isObject(item) || !isObject(item.target)) {
      throw new Error("server returned a list item without a semantic target");
    }
    const target = isObject(item.target.Snapshot)
      ? item.target.Snapshot
      : isObject(item.target.Revision)
        ? item.target.Revision
        : undefined;
    if (target === undefined) {
      throw new Error("server returned an unsupported list target");
    }
    const entityId = canonicalId(target.entity, "ent_", 16);
    if (entityId === undefined) throw new Error("server returned a list target without an entity");
    const snapshotId =
      canonicalId(target.snapshot, "snp_", 32) ?? canonicalId(envelope?.snapshot, "snp_", 32);
    if (snapshotId === undefined)
      throw new Error("server returned a list target without a snapshot");
    return {
      ref: {
        entityId: entityId as ObjectDescription["summary"]["ref"]["entityId"],
        snapshotId: snapshotId as ObjectDescription["summary"]["ref"]["snapshotId"],
        ...(canonicalId(target.revision, "rev_", 32) !== undefined
          ? {
              revisionId: canonicalId(target.revision, "rev_", 32) as NonNullable<
                ObjectDescription["summary"]["ref"]["revisionId"]
              >,
            }
          : {}),
      },
      typeId: (canonicalText(item.entity_type) ??
        "unknown") as ObjectDescription["summary"]["typeId"],
      ...(typeof item.label === "string" ? { label: item.label } : {}),
    };
  });
  const cursor = page.next;
  return {
    items,
    ...(typeof cursor === "string" ? { nextCursor: cursor } : {}),
  };
}

/** Converts a Rust fixed-width ID byte array to its canonical text identity.
 * @param value Wire value containing a string or byte array.
 * @param prefix Type prefix to add for byte arrays.
 * @param byteLength Expected byte array width.
 * @returns The canonical ID string, when the value has a supported form.
 */
function canonicalId(value: unknown, prefix: string, byteLength: number): string | undefined {
  if (typeof value === "string") return value;
  if (isObject(value)) value = value.bytes;
  if (
    !Array.isArray(value) ||
    value.length !== byteLength ||
    !value.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255)
  ) {
    return undefined;
  }
  const hex = value.map((byte: number) => byte.toString(16).padStart(2, "0")).join("");
  return `${prefix}${hex}`;
}

/** Reads a string wrapper emitted by a Rust protocol newtype.
 * @param value Untrusted string or newtype wrapper.
 * @returns The wrapped string, when present.
 */
function canonicalText(value: unknown): string | undefined {
  if (typeof value === "string") return value;
  if (isObject(value) && typeof value.value === "string") return value.value;
  return undefined;
}

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
  if (!isObject(record.reference) && isObject(record.operation)) {
    return normalizeOperation(record.operation);
  }
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
  const statusOperation = isObject(operation?.operation) ? operation.operation : operation;
  const reference = statusOperation?.reference as Record<string, unknown> | undefined;
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
  const catalogRevision = value.catalog_revision ?? value.catalogRevision;
  const upstreamRuntimeRevision = value.upstream_runtime_revision ?? value.upstreamRuntimeRevision;
  const providerResponseId = value.provider_response_id ?? value.providerResponseId;
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
    ...(typeof catalogRevision === "number" ? { catalogRevision } : {}),
    ...(typeof upstreamRuntimeRevision === "string" ? { upstreamRuntimeRevision } : {}),
    ...(typeof providerResponseId === "string" ? { providerResponseId } : {}),
    ...(profile ? { profile } : {}),
    ...(activeComponents ? { activeComponents } : {}),
    ...(appliedSettings !== undefined ? { appliedSettings } : {}),
    ...(typeof appliedSettingsFingerprint === "string" ? { appliedSettingsFingerprint } : {}),
    messages: Array.isArray(value.messages)
      ? (value.messages as ModelRequestDetailResponse["messages"])
      : [],
    tools: Array.isArray(value.tools) ? (value.tools as ModelRequestDetailResponse["tools"]) : [],
    truncated: value.truncated === true,
    contentComplete: value.content_complete === true || value.contentComplete === true,
    ...(Array.isArray(value.fragments)
      ? {
          fragments: value.fragments.filter(isModelRequestDetailFragment).map((fragment) => ({
            kind:
              fragment.kind === "tool_call" || fragment.kind === "toolCall"
                ? "toolCall"
                : (fragment.kind as "message" | "tool"),
            index: fragment.index as number,
            ...(typeof (fragment.parent_index ?? fragment.parentIndex) === "number"
              ? { parentIndex: (fragment.parent_index ?? fragment.parentIndex) as number }
              : {}),
            field:
              fragment.field === "constrained_sampling" || fragment.field === "constrainedSampling"
                ? "constrainedSampling"
                : fragment.field === "tool_name" || fragment.field === "toolName"
                  ? "toolName"
                  : (fragment.field as NonNullable<
                      ModelRequestDetailResponse["fragments"]
                    >[number]["field"]),
            fieldOffset: (fragment.field_offset ?? fragment.fieldOffset) as number,
            ...(typeof fragment.role === "string" ? { role: fragment.role } : {}),
            text: fragment.text as string,
          })),
        }
      : {}),
    ...(typeof value.offset === "number" ? { offset: value.offset } : {}),
    ...(typeof (value.total_chars ?? value.totalChars) === "number"
      ? { totalChars: (value.total_chars ?? value.totalChars) as number }
      : {}),
    ...(typeof (value.next_offset ?? value.nextOffset) === "number"
      ? { nextOffset: (value.next_offset ?? value.nextOffset) as number }
      : {}),
  };
}

/** Checks whether an unknown value is one valid request detail fragment.
 * @param value Unknown fragment candidate.
 * @returns Whether the candidate contains the required page fields.
 */
function isModelRequestDetailFragment(value: unknown): value is Record<string, unknown> {
  if (!isObject(value)) return false;
  const fieldOffset = value.field_offset ?? value.fieldOffset;
  return (
    (value.kind === "message" ||
      value.kind === "tool_call" ||
      value.kind === "toolCall" ||
      value.kind === "tool") &&
    typeof value.index === "number" &&
    [
      "content",
      "tool_name",
      "toolName",
      "name",
      "arguments",
      "description",
      "parameters",
      "constrained_sampling",
      "constrainedSampling",
    ].includes(String(value.field)) &&
    typeof fieldOffset === "number" &&
    typeof value.text === "string"
  );
}
