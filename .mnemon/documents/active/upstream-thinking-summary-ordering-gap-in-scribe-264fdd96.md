---
id: "264fdd96-8b8c-4779-8241-0e26ad976e68"
title: "Upstream thinking-summary ordering gap in ScribeRig event adapter"
description: "Investigation result: rpi-agent's thinking events cannot distinguish provider-visible reasoning summaries from raw thinking, so no ordered ReasoningSummary activity can be emitted safely; includes trace, evidence, and the required upstream API gap."
status: "active"
created_at: "2026-10-21T11:29:29.616Z"
updated_at: "2026-10-21T11:29:29.616Z"
content_hash: "e9041e46a48297fc74cd981810008cb9457d00d8758a44c2a54e9e2eabc1cd2e"
source_paths:
  - "crates/scriberig-agent-runtime/src/upstream_invocation.rs"
  - "crates/scriberig-app/src/event.rs"
  - "crates/scriberig-app/src/operation_agent.rs"
  - "crates/scriberig-session/src/execution.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-ai/src/providers/openai_responses.rs"
session_ids:
  - "48bda424-bbcb-4207-b004-4f4fabf610af"
memory_body_ids:
  []
---

# Upstream thinking-summary ordering gap (ScribeRig)

Date: 2026-10-21. Scope: trace how upstream `rpi-agent` thinking events reach
ScribeRig branch activities and decide whether ordered, provider-visible
reasoning summaries can be published without exposing raw chain-of-thought.
Outcome: **no source change was safe** — the pinned upstream API cannot
distinguish safe summaries, so the task was reported as an API gap instead of
being implemented by inferring from text.

## Trace

- `rpi-agent::AgentEvent::MessageUpdate` carries
  `rpi_ai::types::AssistantMessageEvent`.
- The only thinking-related variants are `ThinkingStart`, `ThinkingDelta`, and
  `ThinkingEnd`; there is no summary-specific event and no visibility
  discriminator.
- `ThinkingEnd.content` / `ThinkingContent.thinking` hold the thinking block
  itself. `ThinkingContent.redacted` only marks safety-redacted opaque payloads
  (with the opaque data in `thinking_signature`), not provider-visible
  summaries.
- In the OpenAI Responses provider
  (`playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-ai/src/providers/openai_responses.rs`),
  both `response.reasoning_summary_text.delta` and
  `response.reasoning_text.delta` map to the same `ThinkingDelta` /
  `ResponseSlotKind::Thinking` slot, so summary and raw reasoning text are
  merged before ScribeRig sees them. `response.reasoning_summary_part.done`
  similarly appends `"\n\n"` into the same slot.
- `crates/scriberig-agent-runtime/src/upstream_invocation.rs` consumes only
  `TextDelta` today; no `AgentActivityKind::ReasoningSummary` producer exists
  anywhere in the app/runtime path, despite the kind, TUI handling, and
  `AgentActivity` event existing.
- `ApplicationEvent::AgentActivity`
  (`crates/scriberig-app/src/event.rs`) carries `operation`, `branch`,
  `activity_kind`, `state`, `summary`, published through
  `OperationRegistry::publish_agent_activity`
  (`crates/scriberig-app/src/operation_agent.rs`, bounds: branch 128 chars,
  summary 256 chars). It has no arrival ordinal.
- The TUI currently derives its activity key from operation + branch + kind, so
  multiple summaries in one invocation would collapse — relevant for the design,
  but TUI rendering was explicitly out of scope.

## Why nothing was implemented

Emitting `ReasoningSummary` from `ThinkingDelta`/`ThinkingEnd` would publish raw
hidden reasoning, and the events carry no field that separates safe summaries
from it. Text-based heuristics were ruled out by the task constraints.

## Required upstream API (gap to close)

1. Typed, provider-owned discriminator: separate
   `ReasoningSummaryStart/Delta/End` events, or a visibility field on thinking
   deltas exposing only safe summary text.
2. Then the ScribeRig event adapter can emit bounded
   `AgentActivity { kind: ReasoningSummary, ... }` from the safe variant only.
3. The narrow activity event/session types can carry a monotonic
   per-invocation ordinal.
4. The TUI can render summaries in arrival order without collapsing them by
   kind/branch.

## Validation

`rustfmt --check` on the inspected adapter/activity types and a scoped
`git diff --check` both pass; no diff was introduced. No tests, builds, commit,
or push (per task constraints). Files changed: none.

Related: the upstream error-flattening constraint at the same pin is documented
in `d637983f-8af5-4854-b5e5-c2c905663e13` — the same pattern (upstream
flattening forces unsafe inference at the ScribeRig boundary) recurs here and
should be fixed in the fork rather than worked around.
