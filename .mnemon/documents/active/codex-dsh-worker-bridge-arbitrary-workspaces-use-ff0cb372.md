---
id: "ff0cb372-fb81-4361-91e2-acb34ba41a1e"
title: "Codex–DSH worker bridge: arbitrary workspaces, user-scope registration, and trusted-root gating"
description: "Implementation handoff for making the Codex→DSH MCP bridge usable from any Codex project: free-form workspace paths, user-level config by default, danger-full-access restricted to trusted roots, and structured rejection payloads because the MCP SDK hides exception text."
status: "active"
created_at: "2026-10-21T12:07:02.808Z"
updated_at: "2026-10-21T12:07:02.808Z"
content_hash: "a358cf5a9746ee9ee4506f2b6281898195a13957982799a572c7ac64dd0c7093"
source_paths:
  - "playground/setup_dsh_codex_worker.py"
  - "playground/.check_scope.py"
  - "playground/.probe_foreign.py"
  - "playground/.probe_envroots.py"
  - "AGENTS.md"
session_ids:
  - "3e3b58be-d7e5-4b23-a14f-74c293f8c125"
memory_body_ids:
  []
---

# Codex–DSH worker bridge: arbitrary workspaces, user-scope registration, trusted-root gating

Date: 2026-10-21. Scope: `D:\prog\ai\scriberig\playground\setup_dsh_codex_worker.py` (generator)
and its installed copy `%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py`.

Related documents:
`645924a4-6252-4b56-8183-0dd9ce6a717c` (bridge architecture, ACP provider override, permission
modes), `93dd0f7e-7b6a-4f14-82ed-4da6c59ceb07` (session discovery/continuation),
`ed5b81da-880f-4c29-a284-99c148c380fb` (delegated sessions appear Ungrouped).

## Why

The bridge was only usable from scriberig: `workspace` was a closed enum (`project` /
`playground`), and setup wrote a project-scoped `[mcp_servers.dsh_worker]` block into
`D:\prog\ai\scriberig\.codex\config.toml`, loaded only for that trusted project. The user wanted
the same delegation available from any Codex project on the machine.

## Changes

- **`workspace` accepts any directory.** `dsh_start`, `dsh_run`, and `dsh_sessions` take
  `"project"`, `"playground"`, `"."`, an absolute path, or a relative path resolved against the
  server's cwd. `"."` means "the directory the MCP server was started from", i.e. the Codex
  project, so a globally registered server can delegate in whatever repository launched it.
- **User-scope registration is the default.** Setup writes the managed block to
  `%USERPROFILE%\.codex\config.toml`; `--scope project` keeps the old behaviour. Setup removes the
  other scope's managed block, so the server is never registered twice. The user-scope entry omits
  `cwd` (inherits Codex's launch directory); the project-scope entry still pins scriberig.
- **`danger-full-access` is gated by trusted roots.** `read-only` and `workspace-write` work for
  any workspace; `danger-full-access` is accepted only under roots defaulting to scriberig plus its
  playground, extensible with `DSH_WORKER_FULL_ACCESS_ROOTS` (semicolon-separated) in the MCP
  server `env` table. The generated config carries that hint as a comment.
- **Expected rejections return a payload instead of raising.** The MCP Python SDK replaces raised
  exception text with the opaque `Error executing tool dsh_run`, so bad paths and untrusted
  full-access requests now return
  `{"status": "rejected", "error": ..., "workspace": ..., "permission_mode": ..., "trusted_roots": [...]}`
  with `isError: false`, letting the controlling agent read the actual reason.
- **Write locks are keyed by resolved directory** rather than by the two former aliases, so
  concurrent write workers serialise per target tree.

## Verification evidence

Probes: `playground/.check_scope.py`, `playground/.probe_foreign.py`, `playground/.probe_envroots.py`.

- Server spawned with cwd `D:\prog\__py_libs__\distlift`: `workspace="."` produced that cwd, DSH
  itself reported it, and `dsh_sessions` was correctly scoped there (0 sessions).
- `danger-full-access` in that directory was rejected with the trusted-roots list; with
  `DSH_WORKER_FULL_ACCESS_ROOTS=D:\prog` the same call succeeded (`TRUSTED-OK`).
- Scope probe results: `project`/`.`/absolute/relative resolve as expected; empty and whitespace
  paths raise `ValueError`; a nonexistent path raises `FileNotFoundError`;
  `danger-full-access` on `D:\prog` raises `PermissionError`.
- Setup output confirmed migration from the project config to
  `C:\Users\Nicu\.codex\config.toml`, removal of the duplicate project block, and that the
  installed copy matches the source.

## Caveats

- `"."` assumes Codex spawns MCP servers with the project as cwd. Where that differs, pass the
  explicit directory path; it always works.
- Codex must be restarted once after setup to pick up user-level config.
- `AGENTS.md` carries the user-facing delegation guidance; the script's module docstring carries the
  operator-facing rationale (ACP vs headless, credential-provider override, Windows shell note).
