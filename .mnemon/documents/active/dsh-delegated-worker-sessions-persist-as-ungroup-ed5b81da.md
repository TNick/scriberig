---
id: "ed5b81da-880f-4c29-a284-99c148c380fb"
title: "DSH delegated worker sessions persist as Ungrouped sessions in the desktop app"
description: "Investigation evidence: every dsh_start/dsh_run ACP job creates a durable DSH session under ~/.dsh/sessions keyed by worker cwd, but never joins its workspace record, so the session browser lists it in the Ungrouped bucket."
status: "active"
created_at: "2026-10-21T11:46:57.913Z"
updated_at: "2026-10-21T11:46:57.913Z"
content_hash: "054e3d0327234c302bf38e6b7fda76617fdd3635f5dd63920f97155214d5711d"
source_paths:
  - "AGENTS.md"
session_ids:
  - "2dd86c1e-259a-4b37-86a4-80206dd83453"
memory_body_ids:
  []
---

# DSH delegated worker sessions persist as "Ungrouped" sessions

Answer to an operational question about the scriberig → DSH ACP worker bridge
(see [DSH ACP worker bridge for Codex](645924a4-6252-4b56-8183-0dd9ce6a717c.md)):
are delegated jobs real resumable sessions, and why do they not appear under the
scriberig workspace in the desktop app?

Verdict: they are **real, persistent, resumable sessions**, but they never become
members of the workspace; the app lists them in the **Ungrouped** bucket.

## On-disk evidence

Six probe runs each produced one session directory:

```
C:\Users\Nicu\.dsh\sessions\--D-prog-ai-scriberig-playground--\
    <session-id>\session.v4.jsonl.zstd
```

Regenerating a workspace-listing probe found 18 `.zstd` files (session
transcripts) and a cached header under
`storages/session_projcache/sessions/<id>.json` recording
`"cwd": "D:\\prog\\ai\\scriberig\\playground"` and
`"sandboxMode": "danger-full-access"`.

- One session per `dsh_start`/`dsh_run` call, keyed by the worker's cwd.
- Each carries a full transcript and survives the worker process, so it can be
  resumed; these are not ephemeral ACP-only sessions.
- Sessions accumulate indefinitely (one per run) unless cleaned up.

## Root cause: workspace membership is a durable record, not a cwd match

From `@deepseek-ai/dsh-workspace` and `@deepseek-ai/dsh-client-ui-workspace`
inside the packaged `app.asar`:

- `Workspace.sessionIds` is authoritative membership — a filtered view of the
  durable `record.sessionIds`; a session joins only through `attachSession`,
  which requires the session's stored cwd to equal the workspace path.
- The ACP bridge calls `session/new` directly on the DSH agent, so
  `attachSession` is never invoked and the workspace record stays unchanged.
- The browser UI's `groupByWorkspace` collects every session not accounted by a
  workspace into a trailing group built with `buildGroup("", …)` and rendered
  with `t("group.ungrouped")`; the group also receives the cwd basename as a
  fallback label (`workspaceLabel`).

Related internals observed while probing:

- `owningGroupKey` resolves a session's group via
  `workspaces.find(w => w.sessionIds.includes(id))`, defaulting to the empty
  string (ungrouped key).
- `WorkspaceRegistry` bootstraps from canonical-cwd session headers and treats
  duplicate path claims or double-accounted sessions as domain corruption, so
  cwd alone cannot imply membership.
- `WorkspaceRegistry` requires the `sessionPersistence` dependency, so an
  unavailable peer is never mistaken for empty history.

## Practical consequences and options

- Delegated runs appear as separate Ungrouped sessions labelled by cwd
  (`playground`/`scriberig`), even with `workspace="project"` — same cwd, still
  unaccounted.
- Each header records `danger-full-access`; sessions pile up over time.
- Cleanup: remove the session directories under
  `~/.dsh/sessions/--D-prog-ai-scriberig-playground--/` while DSH is closed to
  avoid a stale index, or archive/delete them in the app.
- Bridge-side fixes, if desired: reuse one session id per workspace (resume
  instead of `session/new`) or delete the session directory after collecting the
  result.

## Probe artifacts

Scratch probes were created and removed in `playground/` (`.probe_asar*.js`);
verification reported `cleanup ok`. Do not introspect the ACP plugin by treating
`...\resources\app.asar\dsh` as a real directory — unpack it instead (same
caveat recorded in the bridge document).
