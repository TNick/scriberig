---
id: "93dd0f7e-7b6a-4f14-82ed-4da6c59ceb07"
title: "Agent-controlled DSH session continuation in the Codex worker bridge"
description: "Implementation handoff: dsh_sessions discovery, session_id continuation via ACP session/resume, and session id surfaced at creation in playground/setup_dsh_codex_worker.py, with verified ACP method support and the active-session caveat."
status: "active"
created_at: "2026-10-21T11:58:05.518Z"
updated_at: "2026-10-21T11:58:05.518Z"
content_hash: "99975607383a6d0ed2bd53a138cd919f5d372dbdd67593c6ba56ee913f31ee02"
source_paths:
  - "playground/setup_dsh_codex_worker.py"
  - "playground/.probe_sessions.py"
  - "playground/.probe_methods.py"
  - "playground/.probe_mcp_sessions.py"
  - "AGENTS.md"
session_ids:
  - "019409c7-8b8a-43df-9008-a3dadaec7c2c"
memory_body_ids:
  []
---

# Agent-controlled DSH session continuation in the Codex worker bridge

Implementation handoff for making session choice the controlling agent's decision: Codex may
continue an earlier DeepSeek Harness session or start a fresh one, is told the session id as soon
as it exists, and can discover past sessions. Source of truth is
`playground/setup_dsh_codex_worker.py` (generator); the installed copy is
`%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py`.

Related documents: `645924a4-6252-4b56-8183-0dd9ce6a717c` (bridge architecture, permission
modes, acp provider override, launcher bug) and `ed5b81da-880f-4c29-a284-99c148c380fb` (why
delegated sessions appear Ungrouped and the then-proposed "reuse one session id per workspace"
fix, now implemented on the bridge side).

## Why

Before this change each `dsh_start`/`dsh_run` call created a new session with no way to continue
prior context and no way to know the id afterwards. The user's requirements were:

- session continuation must be a choice of the controlling agent (Codex), not an automatic policy;
- the agent needs a way to discover sessions;
- the agent must be informed by session id when a new session is created.

## ACP method support (measured, not assumed)

Probed with `playground/.probe_methods.py` against `dsh --profile acp` and the
`agent-client-protocol` 0.12.1 Python client:

| Method | Result |
| --- | --- |
| `session/new` | works, returns a `sessionId` |
| `session/list` | works (returned 7–10 real sessions, `{'sessionId', 'cwd', ...}`) |
| `session/resume` | works, but fails with `Invalid params: session is already active: <id>` if that session is live in another process |
| `session/load` | `RequestError "Method not found": session/load` |
| `session/fork` | `RequestError "Method not found": session/fork` |

The initial implementation used `session/load` (the name in the SDK's `load_session` helper) and
failed at runtime; `resume_session` is the correct call for DSH. The final code resumes through
ACP `session/resume` and only falls back to `session/load` when an agent reports "method not
found", recording which path was used as `attach_mechanism` (`resume`/`load`).

Stray non-JSON lines on the ACP transport produce `Error parsing JSON-RPC message` /
`JSONDecodeError: Expecting value: line 1 column 2` in the worker log; this is stdout noise from
the child process and is not fatal, but it pollutes probe output (see the earlier bridge document).

## Bridge changes

- **`dsh_sessions(workspace)`** — new MCP tool. Lists existing DSH sessions for a workspace
  (id, title, updated_at, cwd), most recent first, so Codex can judge whether continuing prior
  work is cheaper than a fresh session.
- **`session_id` parameter on `dsh_start` and `dsh_run`** — omit for a new session, pass one to
  continue. Validated by `_validate_session_id`; an invalid id fails before a job is created.
- **Session id reported at creation** — `dsh_start` is now async and waits up to 30 s on a
  `session_ready` event, returning `session_id` and `mode: "new" | "continue"` in its first
  response. The same `session_ready` event is released in the job's `finally` block even when
  session creation fails, so a caller learns about the failure instead of waiting out the window.
- **Job registry carries the session** — `WorkerJob` gained `session_id` and `continued_session`;
  `_job_view` exposes both (plus `requested_session_id` on the start response) so `dsh_status`
  and `dsh_jobs` show the id as soon as it is known. `dsh_start` also reports `mode`.
- Results carry `session_id`, `continued_session`, and `attach_mechanism`.
- The managed AGENTS.md block documents the delegation-time choice: continue only when the
  follow-up genuinely builds on that session, keep the same workspace, otherwise start fresh to
  avoid stale context.

## Verification evidence

- `python -m py_compile` syntax check plus a grep of `session_id` references after each edit.
- Continuation probe (`playground/.probe_sessions.py` equivalent): a new session stored a marker,
  then a second call with that `session_id` answered with the marker content and
  `continued_session: true`.
- `session/list` count grew by exactly one per new session (8 → 9 → 10).
- MCP `tools/list` probe (`playground/.probe_mcp_sessions.py`) confirmed `session_id` in both
  `dsh_start` and `dsh_run` schemas, `dsh_sessions` present, and an end-to-end run where
  `dsh_start` returned `session_id: 67e830bc-…` while the job was still `running`, `dsh_status`
  kept the same id, the job finished with `STARTED-OK`, and a continued `dsh_run` reported the
  same session.
- Re-running the setup regenerated `.codex/config.toml` and AGENTS.md, and the installed copy at
  `%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py` byte-matches the generated source.

## Caveat

DSH supports `session/resume` only when the session is not already active in another process, so
two Codex workers cannot drive one session simultaneously. Existing per-workspace write locks
cover the common case, but the resume call surfaces the conflict as an explicit error rather than
queueing.
