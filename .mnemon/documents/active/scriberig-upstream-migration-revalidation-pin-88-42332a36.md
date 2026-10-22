---
id: "42332a36-6d53-43d6-91a6-6c289e8eb127"
title: "ScribeRig upstream migration revalidation: pin 88e0fa8, safe reasoning-summary bridge, and audit corrections"
description: "Reusable status/evidence snapshot for the ScribeRig→upstream-fork migration: current fork pin and divergence, corrected ownership audit items G6/G10, the safe ReasoningSummaryDelta bridge design, plugin-activation gaps, and what was actually validated."
status: "active"
created_at: "2026-10-21T12:38:51.329Z"
updated_at: "2026-10-21T12:38:51.329Z"
content_hash: "5ddc02289109b5b3feca7465edecf2d2275a0c9996008ff8a9d403e27315fe5f"
source_paths:
  - "design/level-3/144. stop local commodity ownership and establish upstream contribution gates.md"
  - "design/level-3/145. replace local model catalog provider auth thinking and custom model properties with upstream.md"
  - "design/level-3/146. replace local provider tool loop and extension host with upstream agent and plugin runtime.md"
  - "design/level-3/147. replace ratatui shell and tui kit bridge with upstream tui runtime and native model controls.md"
  - "design/level-3/148. rebuild scriberig semantic session plan and review surfaces as upstream tui components.md"
  - "design/level-3/149. remove superseded local infrastructure prove delegation and finish the correction.md"
  - "crates/scriberig-agent-runtime/src/lib.rs"
  - "crates/scriberig-agent-runtime/src/upstream_invocation.rs"
  - "apps/scriberig/src/native_tui_agent/runner/summary_activity.rs"
  - "apps/scriberig/src/native_tui_agent/runner.rs"
  - "crates/scriberig-app/src/event.rs"
  - "crates/scriberig-app/src/operation_agent.rs"
  - "crates/scriberig-tui/src/state_actions/agent_activity.rs"
session_ids:
  - "e7323260-df6a-4191-a63c-743b867990a8"
memory_body_ids:
  []
---

# ScribeRig upstream migration — 2026-10-21 revalidation

Repository: `D:\prog\ai\scriberig`, branch `design/adopt-rpi-tui`.
Controlled fork: `playground/agentic-work/20261021-204000 pi-tree-virtualization`,
branch `codex/scriberig-upstream-ownership`.

## Pin and divergence (authoritative)

- Every `rpi-*` dependency and `Cargo.lock` resolves to fork commit
  `88e0fa8972df4be3ad5313b245a78c73d8bc1e29` (tracking `TNick/pi-rust`).
  Older task notes naming `b2fc63c…` / `bad1c5b…` / `b78a199…` are stale.
- Official `bigfish1913/pi-rust` `main` was `281dfddfa39ac063d86de57b75370974c4d75e6c`
  (0.7.1). The fork is **0 behind, 62 ahead** of it: no upstream merge was
  needed, no conflicts, no pin movement; the fork already contains the 0.7.1
  merge plus ScribeRig catalog, stream, TUI, reasoning-summary, worker, and
  sandbox work.
- The isolated worker, sandbox launcher, `ReasoningSummaryDelta`, and
  `TreeList::{items, replace_items}` are **committed and pinned**, not
  uncommitted working-tree work.

## Corrected ownership-audit items (task 144)

`design/level-3/144…` is complete as an ownership inventory / contribution-gate
audit, not as evidence that the migration finished. Its §12.8 "late
revalidation" (from line ~685) carries the current facts:

- **G6 narrowed**: upstream now owns dynamic tree item replacement
  (`TreeList::items()`, validated `replace_items()`). Grouping, generic table,
  and resizable split remain upstream gaps.
- **G10 narrowed**: upstream owns the process/IPC boundary, host allowlist, and
  launched sandbox contract. ScribeRig owns host policy
  (`ApplicationDomainDispatcher`, `PluginHostPolicy`, `AuthorizedPluginWorker`)
  and does **not** compose the worker into production.
- Receipt/auth classification is still partly string-typed: the fork exposes
  `AiError::Auth`, but the agent stream still flattens provider errors into
  assistant-message text.
- No new local commodity owner was introduced; the summary change is an adapter
  over a typed upstream event.
- Current dependency headers of tasks 145–149 were updated to stop citing the
  old pins; those tasks remain In Progress.

## Safe reasoning-summary bridge (implemented)

Design rule: publish only provider-visible reasoning summaries, never raw
chain-of-thought. The pinned fork now separates
`AssistantMessageEvent::ReasoningSummaryDelta` from `ThinkingDelta` /
`ThinkingEnd`, and routes OpenAI Responses `reasoning_summary_text.delta` to the
former and `reasoning_text.delta` to the hidden thinking slot.

Implementation:

- `crates/scriberig-agent-runtime/src/lib.rs` — `ReasoningSummaryCallback`
  (`Arc<dyn Fn(u64, &str)>`), first argument is a per-invocation ordinal.
- `crates/scriberig-agent-runtime/src/upstream_invocation.rs` (~L116–130) —
  forwards **only** `ReasoningSummaryDelta`; `ThinkingDelta`, `ThinkingEnd`, and
  their content are never forwarded.
- `apps/scriberig/src/native_tui_agent/runner/summary_activity.rs` — bounded,
  cumulative `ReasoningSummaryActivity` sink emitting Running progress plus a
  final Completed event.
- Ordinal propagates through `ApplicationEvent::AgentActivity`
  (`crates/scriberig-app/src/event.rs`), server metadata
  (`scriberig-server/src/server_helpers_event.rs`), and TUI state
  (`state_actions/agent_activity.rs`); the TUI drops summaries with a lower
  ordinal. Rendering/visibility behavior was deliberately not changed.
- Child and plan-worker executions pass `None`, so child summaries cannot be
  misattributed to the root branch; the CLI one-shot path also leaves it unset.

Remaining limitation: there is no separate start/end event, no per-part
identity, no full-vs-summary visibility distinction, and no durable
reasoning-segment model. The earlier gap analysis (document
`264fdd96-8b8c-4779-8241-0e26ad976e68`) described the pre-fork state where no
source change was safe; the pinned fork's typed split is what unblocked this
adapter.

## Plugin activation status

Present and pinned: versioned worker protocol, process ownership,
correlation/deadlines, cooperative cancellation, shutdown, host-call RPC,
process-tree termination on host timeout; injected `WorkerLauncher`, default
environment clearing, fail-closed `HostProcessLauncher`,
`LinuxBubblewrapLauncher`; ScribeRig `AuthorizedPluginWorker` enforcing a
sandboxed policy with `AllowlistedHostHandler`.

Not production-ready: `validate_isolated_plugin_configuration` rejects every
configured worker/package root, so the worker never starts and worker tools never
enter `ToolCatalog`; sandbox guarantees are launcher self-reports, not
independently verified; no macOS/Windows launcher, seccomp, or CPU/memory
limits; host capability calls are not cancellation-aware; no atomic
extension-session/catalog reload. Plugin activation therefore remains
deny-by-default. See documents
`396513a1-8417-44f4-b544-ba0462223293` and
`70cd80f1-35d4-46fd-ad5c-c95859f358a7` for the boundary and lifecycle handoffs.

## Validation actually performed

- `cargo fmt --all` and `cargo fmt --all -- --check` — passed.
- Scoped `git diff --check` over changed source/design files — passed. The only
  repository-wide findings were pre-existing trailing-whitespace lines in
  user-owned `AGENTS.md`, which was not modified.
- `cargo check -p scriberig --bin srig --locked` — passed (~21.8 s).
- Not run: tests, workspace-wide check, clippy, PTY/golden acceptance.

## Remaining work / risks

No hard blocker. Tasks 145–149 remain In Progress. Largest open product risks:
plugin activation with enforced isolation, complete effective-request
provenance and child/plan-worker continuation semantics, and PTY/behavioral
acceptance of the semantic TUI session/plan/review surfaces.
