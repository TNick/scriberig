---
id: "51700d7e-ff71-48a7-8d9b-d78ca6519b26"
title: "Self-hosting onboarding loop: HTML-comment Markdown rejection and masked import diagnostics"
description: "Root-cause investigation and fix handoff for the ScribeRig self-hosting onboarding loop (Markdown adapter rejecting HTML comments), tool-error masking, child-result parsing, and the remaining agent-continuation delegation blocker."
status: "active"
created_at: "2026-10-22T09:58:12.547Z"
updated_at: "2026-10-22T09:58:12.547Z"
content_hash: "c9bf311f90eab91f5bbd3b61441866ad9bdb713cbe5a85172e565f83a033f432"
source_paths:
  - "apps/scriberig/src/self_hosting_provider/branches.rs"
  - "apps/scriberig/src/self_hosting_provider/branches/result_helpers.rs"
  - "apps/scriberig/src/native_tui_agent/upstream_execution/delegation.rs"
  - "crates/domains/sr-markdown"
  - "apps/scriberig/tests/tui_pty_support/onboarding_wait.rs"
session_ids:
  - "c7b850f7-6e49-4e05-a673-1c4c0747e63a"
memory_body_ids:
  []
---

# Self-hosting onboarding loop (ScribeRig)

Date: 2026-10-22. Scope: why `tui_self_hosts_php_candidate_without_changing_live_checkout`
(and the wider `tui_pty_self_hosting*` family) never completed onboarding, and what was
fixed. The upstream-first ownership boundary was preserved; no ScribeRig commit/push.

## Root cause (fixed)

`project_import` failed because the Markdown adapter (`crates/sr-markdown`) rejected
HTML/Inline-HTML events found in `AGENTS.md` as `unsupported-markdown`. The agent then
retried onboarding until the faux response queue drained, producing the endless loop and
finally `branch "agent continuation" terminated as Abandoned: Some("upstream agent: No more
faux responses queued")`.

Two masking layers hid this diagnostic, so the retry looked like a provider problem:

1. The semantic broker (`broker.rs`) replaced the real tool-error category/message.
2. The upstream tool adapter (`upstream_tool.rs`) flattened it again.

Fix: HTML/Inline-HTML events are ignored instead of rejecting the document, with an
`sr-markdown` regression test for HTML comments; both masking layers now preserve the real
bounded category/message (`import_diagnostics: AGENTS.md: unsupported-markdown` became
visible), which is what localized the defect.

## Self-hosting scenario parsing (fixed)

`apps/scriberig/src/self_hosting_provider/branches/result_helpers.rs` previously recognized
only the legacy `child_summary=… handles=[…]` text form. The current provider emits a JSON
child-result shape (`summary`, `branch`, `branch_result`, `handles`). The helpers now accept
the JSON shape as well, via `child_summary`, `is_child_result`, and
`is_successful_candidate_validation`; the file was extracted out of `branches.rs` and still
stays under the 400-line rule.

## Remaining blocker (open)

Onboarding now completes, but the following `agent continuation` branch is still abandoned.
Captured tool messages contain only `runtime_capabilities`, `semantic_find`, and render
results — no completed child `BranchResult` — so the parent re-delegates until the faux pool
is exhausted. Evidence suggests the delegated child does produce a result, but the parent's
child-result detection/compaction path does not consume it, and the propagated output is very
large instead of compact. Next investigative target: `apps/scriberig/src/native_tui_agent/upstream_execution/delegation.rs`
(the `complete_local_child_agent_with_evidence` / `fail_local_child_agent_with_evidence`
result-handle chain around lines 100–165), then the same path for implementation children.

## Verification at handoff

- `tui_pty`: 28/28 when run per test; a full-suite run reported 26/28 with two timing flakes
  (`arithmetic`, `abandoned conversation`) that pass individually.
- `tui_pty_golden_agent`: 1/1.
- `sr-markdown` HTML-comment regression: passing.
- Workspace clippy `-D warnings`, fmt, and `tools/lint_repo.py` lint: passing.
- `tui_pty_self_hosting*` targets: not green (blocked by the item above).

## Lessons

- A tool-error masking layer turns a one-line adapter diagnostic into an apparent agent loop;
  preserve bounded category + message through every adapter hop.
- Test helpers that assert on serialized child results must track the current wire shape;
  a stale matcher silently disables assertions rather than failing loudly.
