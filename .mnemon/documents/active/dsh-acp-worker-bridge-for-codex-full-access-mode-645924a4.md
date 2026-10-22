---
id: "645924a4-6252-4b56-8183-0dd9ce6a717c"
title: "DSH ACP worker bridge for Codex: full-access mode, provider override, and launcher fixes"
description: "Operating notes and implementation handoff for the Codex→DSH ACP worker bridge in scriberig: danger-full-access support, acp profile provider override, cmd /q launcher fix, and known stdout noise."
status: "active"
created_at: "2026-10-21T11:41:55.266Z"
updated_at: "2026-10-21T11:41:55.266Z"
content_hash: "c22e52a8ab626a495ef89ab8ab0f5a8583676d7d5da7149d90cf5995fde4b010"
source_paths:
  - "playground/setup_dsh_codex_worker.py"
  - "playground/.probe_mcp.py"
  - "playground/.probe_full_access.py"
  - "AGENTS.md"
session_ids:
  - "f9d10ce4-799e-44c6-ab78-3b18a249b4aa"
memory_body_ids:
  []
---

# DSH ACP worker bridge for Codex

Date: 2026-10-21. Scope: `D:\prog\ai\scriberig\playground\setup_dsh_codex_worker.py` and the local
`~/.dsh` profile configuration. This records why the bridge is configured the way it is; the
generated bridge script lives at `%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py`.

## Architecture at a glance

Codex reaches DSH through two surfaces:

1. An MCP stdio server (`dsh_worker.py`, installed to `%LOCALAPPDATA%\CodexDshWorker\scriberig\dsh_worker.py`)
   exposing `dsh_start`, `dsh_run`, `dsh_status`, `dsh_jobs`.
2. An ACP child process launched per job: `dsh --profile acp`, driven over JSON-RPC by the
   Python `acp` / `agent-client-protocol` 0.12.1 client.

Setup writes `D:\prog\ai\scriberig\.codex\config.toml` and a managed block in
`D:\prog\ai\scriberig\AGENTS.md` (`<!-- BEGIN managed DSH worker instructions -->`).

## Permission modes

`read-only`, `workspace-write`, and `danger-full-access` are all advertised by `dsh_start` and
`dsh_run` (verified by an MCP `tools/list` probe). `dsh_status` and `dsh_jobs` carry no
`permission_mode` property — tool schemas must not assume it. `danger-full-access` jobs serialize
through `_job_lock` exactly like write jobs.

Probe scripts: `playground/.probe_mcp.py`, `playground/.probe_raw_line.py`, plus the end-to-end
`playground/.probe_full_access.py`.

## Failure: acp profile pointed at an unkeyed provider

The shipped `acp` profile (`@deepseek-ai/dsh-base` + `@deepseek-ai/dsh-acp-app` bundles) defaults
its `acp` entry to `provider: deepseek-official`, `model: deepseek-v4-flash`. No API key exists for
`deepseek-official`, so every worker turn died with:

`Internal error: turn failed: llm-deepseek: no API key for provider route "deepseek-official"`

This machine's GUI runs the `desktop` profile on `deepseek-account`, which is credentialed. Fix is a
user-layer override appended to `C:\Users\Nicu\.dsh\profiles\acp\cordis.patch.yml` (originals backed
up as `cordis.patch.yml.bak`):

```yaml
- id: acp
  name: "@deepseek-ai/dsh-acp"
  config:
    provider: deepseek-account
    model: deepseek-flash
```

Verified in `dsh --profile acp --dump-config`, which should show the `acp` entry patched by
`cordis.patch.yml` with `provider: deepseek-account`. Patch layers compose after every bundle layer
in order: bundles from `package.json` `dsh.profile.bundles`, then `cordis.patch.yml`, then `--patch`
overlays. Edit the patch file, never the generated `cordis.yml` (an empty `[]` root by design).

## Bug: cmd batch launcher split on the space in "DeepSeek Harness"

Invoking the launcher as a pre-quoted `/s /c "<path>" args` line silently broke because the DSH
install path contains a space (`C:\Users\Nicu\AppData\Local\Programs\DeepSeek Harness\...`). Correct
form: `cmd /d /q /c <path> <args...>` — no `/s`, no manual pre-quoting; `/q` also suppresses the
echo of the batch invocation onto stdout.

Do not introspect the ACP plugin by walking the packaged `app.asar` tree: `...\resources\app.asar\dsh`
does not exist as a real directory. The usable launcher is
`C:\Users\Nicu\AppData\Local\Programs\DeepSeek Harness\resources\runtime\cli\bin\dsh.cmd`.

## Bug: agent-client-protocol 0.12.1 content blocks

Prompt content must be built as `TextContentBlock(type="text", ...)`; the older bare form is
rejected by 0.12.1.

## Verification

- `setup_dsh_codex_worker.py --setup`: DSH found, venv at `%LOCALAPPDATA%\CodexDshWorker\scriberig\venv`,
  self-test OK, Codex config and AGENTS.md block written.
- MCP initialize handshake OK; enum lists above confirmed.
- Real ACP run with `danger-full-access` returned `FULL-ACCESS-OK` with `stop_reason: end_turn`.

## Known cosmetic noise (not fixed)

`@deepseek-ai/dsh-deepseek-account` prints
`[llm-deepseek-account] model listing from https://api.deepseek.com/models loaded 2 model(s)...`
to stdout. ACP parses stdout as JSON-RPC, so each run logs one
`ERROR:root:Error parsing JSON-RPC message ... JSONDecodeError: Expecting value: line 1 column 2`
before continuing. Harmless, but worker stderr is never clean; don't chase it as a failure signal.
A run is successful when the JSON result block with `"status": "completed"` appears and exit is 0.

## Operational notes

- A new Codex session in the repo is required to pick up the `dsh_worker` MCP server.
- For read-only investigation, prefer `workspace="project"` + `read-only`; for scratch work use
  `workspace="playground"` + `workspace-write`. Reserve `danger-full-access` for explicitly
  authorized work and say why.
- Scriberig policy requires temporary agent work under `playground/agentic-work/` with a
  `YYYMMDD-HHMMSS <name>` directory, removed when done.
