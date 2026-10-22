---
id: "d637983f-8af5-4854-b5e5-c2c905663e13"
title: "Typed authentication-failure propagation from upstream branch invocation"
description: "Implementation handoff for AgentError::Authentication / UpstreamBranchError::Authentication, including the upstream error-flattening constraint, text-prefix classification, and known gaps."
status: "active"
created_at: "2026-10-21T11:09:56.728Z"
updated_at: "2026-10-21T11:09:56.728Z"
content_hash: "d835c6f4a4578a2b300740973888a8f2274bf715a8a8bc95b1b67a055e1a801c"
source_paths:
  - "crates/scriberig-agent-runtime/src/lib.rs"
  - "crates/scriberig-agent-runtime/src/upstream_invocation.rs"
  - "crates/scriberig-agent-runtime/src/upstream_invocation/types.rs"
  - "apps/scriberig/src/main_agent_execution/upstream.rs"
  - "apps/scriberig/src/native_tui_agent/test_execution.rs"
session_ids:
  - "06337231-aa7e-43f5-9806-acc0e2a684a8"
memory_body_ids:
  []
---

# Typed authentication-failure propagation from upstream branch invocation

Date: 2026-10-21. Scope: distinguish upstream provider authentication failures
from ordinary `AgentError::Provider` failures across the branch-invocation
boundary. No fork/upstream files, isolated-worker files, or task-149 notes were
edited.

## Change

Five ScribeRig files, 67 insertions / 5 deletions:

- `crates/scriberig-agent-runtime/src/lib.rs` — `AgentError::Authentication(String)`
  variant, `AgentError::is_authentication()`, and distinct `Display` text
  (`agent authentication: …`).
- `crates/scriberig-agent-runtime/src/upstream_invocation/types.rs` —
  `UpstreamBranchError::Authentication(String)` with display text
  `upstream authentication: …`.
- `crates/scriberig-agent-runtime/src/upstream_invocation.rs` — new
  `agent_failure(message)` / `is_authentication_error(message)` helpers classify
  upstream failure text at the three points where errors surface (the run
  future, `state.error_message`, and `assistant.error_message`).
- `apps/scriberig/src/main_agent_execution/upstream.rs` and
  `apps/scriberig/src/native_tui_agent/test_execution.rs` — map
  `UpstreamBranchError::Authentication` to `AgentError::Authentication` in both
  execution adapters.

## Rationale / constraint

This is **not** typed propagation from `rpi-ai`. At the current fork pin,
`rpi_ai::AiError` is flattened into the assistant message's `error_message`
string before reaching the adapter, so the only available signal is display
text. Classification therefore matches the direct `AiError::Auth` form
(`auth error: …`) and the `rpi-agent` provider wrapper
(`provider error: auth error: …`) after leading-whitespace trim.

## Known gaps

- `AiError::Auth` is emitted only for invalid protected-provider credentials;
  HTTP 401/403 arrive as `AiError::Http` and remain ordinary provider failures,
  so those callers still cannot detect authentication problems.
- If upstream later exposes a structured error kind/code on
  `AssistantMessageEvent`, the text-prefix match should be replaced by that
  typed field.
- Callers that only stringify `AgentError` lose the distinction; they must match
  the variant or call `is_authentication()`.
- No regression test was added for the new classification/`is_authentication()`
  behavior, which the repository convention expects for a defect fix.

## Validation

`rustfmt --check` and `git diff --check` pass on the five files. `cargo check`,
clippy, tests, and builds were not run; nothing was committed.
