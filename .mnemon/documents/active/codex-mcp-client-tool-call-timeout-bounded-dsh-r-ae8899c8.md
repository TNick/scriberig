---
id: "ae8899c8-5cee-44cd-9200-c9d678ae705c"
title: "Codex MCP client tool-call timeout: bounded dsh_run/dsh_wait protocol in the DSH worker bridge"
description: "Why Codex aborts MCP tool calls at 300s regardless of worker timeout_seconds, the exact Codex config keys that change it, and the bounded-start/poll protocol (dsh_run returns running + job_id, dsh_wait long-polls ≤280s) implemented in the DSH worker bridge."
status: "active"
created_at: "2026-10-21T12:13:31.182Z"
updated_at: "2026-10-21T12:13:31.182Z"
content_hash: "a8545777757d8b9c2cbaaf3a0539f1cecaf966aaf144fac737e368eba38221fd"
source_paths:
  - "playground/setup_dsh_codex_worker.py"
  - "playground/.probe_wait.py"
  - "AGENTS.md"
session_ids:
  - "f073139c-d19c-475a-98ab-c691cc11b613"
memory_body_ids:
  []
---

# Codex MCP client tool-call timeout and the bounded job/wait protocol

Date: 2026-10-21. Scope: `playground/setup_dsh_codex_worker.py` (generator) and its installed copy
`%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py`. This records a failure mode that looked
like a DSH worker bug but was a *client-side* cap, and the bridge changes that make long work
survive any client timeout.

Related documents: `645924a4-6252-4b56-8183-0dd9ce6a717c` (bridge architecture and ACP child),
`93dd0f7e-7b6a-4f14-82ed-4da6c59ceb07` (session discovery/continuation),
`ff0cb372-fb81-4361-91e2-acb34ba41a1e` (arbitrary workspaces, user-scope registration,
trusted-root gating, structured rejection payloads).

## The symptom and the real cause

A long delegated task (an audit with `timeout_seconds: 1800`) failed with
`timed out awaiting tools/call after 300s`. The 300s limit belongs to the **MCP client**, not the
worker: `dsh_run`'s `timeout_seconds` only bounds the DSH turn. So the worker kept running while
Codex had already abandoned the response — the work was lost with the MCP server process, not
because DSH failed.

Codex 0.163.0 exposes the cap per server. Keys were verified against the binary's own strings
(`cli/src/mcp_cmd.rs`):

- `startup_timeout_sec`
- `tool_timeout_sec`

The managed `[mcp_servers.dsh_worker]` block now sets `startup_timeout_sec = 60` and
`tool_timeout_sec = 600`. Codex must be restarted once to re-read `%USERPROFILE%\.codex\config.toml`.
Raising the client cap is only a mitigation: the bridge must not depend on any particular cap.

## Design rule: no tool call outlives the client

Every MCP tool call in the bridge is bounded internally at **≤280s**, clamped regardless of what
the caller requests (`MAX_WAIT_SECONDS`, `_validate_wait`). Real work always lives in background
jobs registered in `_JOBS` and keyed by `job_id`, so an early return loses nothing:

- **`dsh_run`** is start + bounded wait. If the job finishes within the wait it returns the final
  result; otherwise it returns `{"status": "running", "job_id": ...}`.
- **`dsh_wait(job_id, wait_seconds)`** (new tool) long-polls up to 280s per call, including partial
  assistant output; call it repeatedly for work that runs for many minutes. Unknown ids return
  `{"state": "not_found", ...}` with recent `known_job_ids`.
- **`dsh_start` + repeated `dsh_wait`** remains the preferred pattern for long work: the `job_id`
  arrives immediately and polling interleaves with the controlling agent's own work.
- `_create_job` and `_wait_for_job` are module-level helpers shared by the start/run tools;
  `_validate_timeout` stays next to them. `SESSION_WAIT_SECONDS = 30` bounds how long `dsh_start`
  waits for a session id before returning without one.

Constants involved: `DEFAULT_TIMEOUT_SECONDS = 900`, `MAX_TIMEOUT_SECONDS = 3600`,
`MAX_BACKGROUND_JOBS = 8`, `MAX_RESULT_CHARS = 60_000`, `MAX_PARTIAL_CHARS = 12_000`.

`AGENTS.md` (managed block) now states the rule explicitly: the MCP client kills a tool call
sooner than the worker budget (Codex: 300s by default), `dsh_run`/`dsh_wait` return
`running` + `job_id` instead of blocking, and a returned `job_id` is durable.

## Verification

`playground/.probe_wait.py` drives the built bridge over stdio JSON-RPC. Notes:

- `tools/list` must be requested as a raw JSON-RPC request; the first probe failed with
  `KeyError: 'tools'` because it indexed the response wrapper instead of the `result` payload.
- The probe confirmed the exposed tool set: `dsh_start`, `dsh_wait`, `dsh_status`, `dsh_jobs`,
  `dsh_sessions`, `dsh_run`.
- `dsh_run(wait_seconds=3)` returned `running` in 3.0s with a `job_id`; `dsh_wait` on that job
  returned `completed` 10.9s later with the full output; an unknown id returned `not_found`.
- Setup self-test passed and the installed copy matched the source; `setup_dsh_codex_worker.py`
  reported the user-scope config write and `AGENTS.md` update; temp probes were cleaned up.

## Operational consequence

A delegated job interrupted by a client timeout dies with its MCP server process and must be
restarted; it cannot be resumed through `dsh_status` in a new Codex process. Because of that,
long work should be started with `dsh_start` and polled, so a client hiccup costs a call rather
than the whole task.
