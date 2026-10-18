/** @file Renders bounded durable branch activity with local view preferences. */

import type { AgentPresentationPreferences, TransientAgentActivity } from "@scriberig/model";
import type {
  BranchActivityPage,
  ModelRequestDetailResponse,
  ToolResultDetailResponse,
} from "@scriberig/protocol";
import { useEffect, useState, type ReactElement } from "react";

const DEPTH_COLORS = ["#4f46e5", "#0891b2", "#16a34a", "#d97706"];

/** Renders durable activity without treating streams as session history.
 * @param props Activity view properties.
 * @param props.page Bounded activity page to render.
 * @param props.presentation Client-local display choices.
 * @param props.onLoadMore Called when another page is requested.
 * @param props.onNavigate Called when a durable handle is opened.
 * @param props.onLoadModelRequest Loads one previously captured request.
 * @param props.onLoadToolResult Loads expanded detail for one durable result.
 * @returns The rendered activity list.
 */
export function BranchActivityView({
  page,
  presentation,
  onLoadMore,
  onNavigate,
  onLoadModelRequest,
  onLoadToolResult,
  transientItems = [],
}: {
  page: BranchActivityPage;
  presentation?: AgentPresentationPreferences;
  onLoadMore?: (cursor: string) => void;
  onNavigate?: (handle: string) => void;
  onLoadModelRequest?: (
    session: string,
    branch: string,
    invocation: string,
  ) => Promise<ModelRequestDetailResponse>;
  onLoadToolResult?: (
    session: string,
    branch: string,
    result: string,
  ) => Promise<ToolResultDetailResponse>;
  transientItems?: readonly TransientAgentActivity[];
}): ReactElement {
  const [requestDetails, setRequestDetails] = useState<Record<string, ModelRequestDetailResponse>>(
    {},
  );
  const [requested, setRequested] = useState<Record<string, true>>({});
  const [resultDetails, setResultDetails] = useState<Record<string, ToolResultDetailResponse>>({});
  const [resultRequested, setResultRequested] = useState<Record<string, true>>({});
  const visibleItems = page.items.filter(
    (item) =>
      presentation?.reasoning === "shown" ||
      (item.activityKind !== "ReasoningSummary" && item.activityKind !== "RuntimeStatus"),
  );
  const visibleTransientItems = transientItems.filter(
    (item) =>
      presentation?.reasoning === "shown" ||
      (item.activityKind !== "ReasoningSummary" && item.activityKind !== "RuntimeStatus"),
  );
  useEffect(() => {
    if (
      presentation?.prompt !== "shown" ||
      onLoadModelRequest === undefined ||
      page.sessionId === undefined
    )
      return;
    const pending = page.items.filter(
      (item) =>
        item.activityKind === "ModelInvocation" &&
        item.handles[0] !== undefined &&
        requestDetails[item.id] === undefined &&
        requested[item.id] === undefined,
    );
    if (pending.length === 0) return;
    setRequested((current) => ({
      ...current,
      ...Object.fromEntries(pending.map((item) => [item.id, true as const])),
    }));
    for (const item of pending) {
      const invocation = item.handles[0];
      if (invocation === undefined) continue;
      void onLoadModelRequest(page.sessionId, page.branchId, invocation)
        .then((detail) => setRequestDetails((current) => ({ ...current, [item.id]: detail })))
        .catch(() => undefined);
    }
  }, [onLoadModelRequest, page, presentation?.prompt, requestDetails, requested]);
  const depthById = new Map<string, number>();
  const depths = visibleItems.map((item) => {
    const depth =
      item.parentId === undefined ? 0 : Math.min(8, (depthById.get(item.parentId) ?? -1) + 1);
    depthById.set(item.id, depth);
    return depth;
  });
  return (
    <section aria-label={`Branch activity ${page.branchId}`}>
      <h2>Branch activity</h2>
      <ol>
        {visibleItems.map((item, index) => {
          const depth = depths[index] ?? 0;
          const isToolInvocation = item.activityKind === "ToolInvocation";
          const isToolResult = item.activityKind === "ToolResult";
          const requestDetail = requestDetails[item.id];
          return (
            <li key={item.id} data-activity-id={item.id}>
              <div style={{ display: "flex", gap: "0.5rem" }}>
                {Array.from({ length: depth }, (_, rail) => (
                  <span
                    key={`${item.id}-rail-${rail}`}
                    aria-hidden="true"
                    style={{
                      width: "2px",
                      minWidth: "2px",
                      flexShrink: 0,
                      backgroundColor: DEPTH_COLORS[rail % DEPTH_COLORS.length],
                    }}
                  />
                ))}
                <div style={{ paddingInlineStart: `${depth}ch` }}>
                  {isToolInvocation && presentation?.toolInvocation === "simple" ? (
                    <code>
                      &gt; {item.summary}
                      {compactToolArguments(item.toolArguments)}
                    </code>
                  ) : isToolInvocation ? (
                    <dl>
                      <dt>Tool invocation</dt>
                      <dd>{item.toolName ?? item.summary}</dd>
                      {Object.entries(item.toolArguments ?? {}).map(([name, value]) => (
                        <div key={`${item.id}-${name}`}>
                          <dt>{name}</dt>
                          <dd>
                            <pre>{JSON.stringify(value, null, 2)}</pre>
                          </dd>
                        </div>
                      ))}
                    </dl>
                  ) : isToolResult && presentation?.toolOutput === "hidden" ? (
                    <span>Tool result hidden</span>
                  ) : isToolResult && presentation?.toolOutput === "full" ? (
                    <>
                      <pre>{JSON.stringify(item.toolResultPreview ?? {}, null, 2)}</pre>
                      <details
                        onToggle={(event) => {
                          if (
                            !event.currentTarget.open ||
                            onLoadToolResult === undefined ||
                            page.sessionId === undefined ||
                            resultDetails[item.id] !== undefined ||
                            resultRequested[item.id] !== undefined
                          )
                            return;
                          const result = item.handles[0];
                          if (result === undefined) return;
                          setResultRequested((current) => ({ ...current, [item.id]: true }));
                          void onLoadToolResult(page.sessionId, page.branchId, result)
                            .then((detail) =>
                              setResultDetails((current) => ({
                                ...current,
                                [item.id]: detail,
                              })),
                            )
                            .catch(() => undefined);
                        }}
                      >
                        <summary>Expand full result</summary>
                        {resultDetails[item.id] ? (
                          <pre>{JSON.stringify(resultDetails[item.id]?.detail, null, 2)}</pre>
                        ) : (
                          <small>Loading durable result detail…</small>
                        )}
                      </details>
                      {resultDetails[item.id]?.truncated ? (
                        <small>Result detail truncated</small>
                      ) : null}
                    </>
                  ) : (
                    <>
                      <span>{item.kind}: </span>
                      <span>{item.summary}</span>
                    </>
                  )}
                  {item.activityKind === "ModelInvocation" && presentation?.prompt === "shown" ? (
                    <details>
                      <summary>Model request</summary>
                      {requestDetail ? (
                        <>
                          {requestDetail.messages.map((message, messageIndex) => (
                            <pre key={`${item.id}-message-${messageIndex}`}>
                              {message.role}: {message.content}
                            </pre>
                          ))}
                          {requestDetail.tools.map((tool) => (
                            <pre key={`${item.id}-tool-${tool.name}`}>
                              {tool.name}: {tool.description}
                              {tool.parameters ? `\n${tool.parameters}` : ""}
                            </pre>
                          ))}
                          {requestDetail.truncated ? <small>Request detail truncated</small> : null}
                        </>
                      ) : onLoadModelRequest === undefined ? (
                        <small>Captured request unavailable</small>
                      ) : (
                        <small>Loading captured request…</small>
                      )}
                    </details>
                  ) : null}
                  {item.handles.length > 0 ? (
                    <small>
                      {item.handles.map((handle) => (
                        <button key={handle} type="button" onClick={() => onNavigate?.(handle)}>
                          Open {handle}
                        </button>
                      ))}
                    </small>
                  ) : null}
                </div>
              </div>
            </li>
          );
        })}
      </ol>
      {visibleTransientItems.length > 0 ? (
        <section aria-label="Live activity">
          <h3>Live activity</h3>
          <ol>
            {visibleTransientItems.map((item) => (
              <li key={item.sequence} data-activity-state={item.state}>
                <strong>{item.activityKind}</strong>: {item.summary}
              </li>
            ))}
          </ol>
        </section>
      ) : null}
      {page.nextCursor && onLoadMore ? (
        <button type="button" onClick={() => onLoadMore(page.nextCursor ?? "")}>
          Load more activity
        </button>
      ) : null}
    </section>
  );
}

/** Formats bounded safe arguments as one compact command suffix.
 * @param values Redacted structured argument object.
 * @returns Compact command parameters for simple invocation mode.
 */
function compactToolArguments(values: Readonly<Record<string, unknown>> | undefined): string {
  if (values === undefined) return "";
  return Object.entries(values)
    .map(([name, value]) => ` --${name} ${JSON.stringify(value)}`)
    .join("");
}
