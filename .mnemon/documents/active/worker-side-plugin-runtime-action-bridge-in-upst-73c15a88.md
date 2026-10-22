---
id: "73c15a88-1530-449c-8a28-9758453c4274"
title: "Worker-side plugin runtime_action bridge in upstream rpi-extensions isolated worker"
description: "Implementation handoff for the worker-side runtime_action bridge (WorkerHostClient, ActionBridge relay, host capability allowlist) and its remaining lifecycle/security gaps."
status: "active"
created_at: "2026-10-21T11:03:05.915Z"
updated_at: "2026-10-21T11:03:05.915Z"
content_hash: "05795f14a03c172d0e3abd049e23665ad287e82ae89407f3e62cdbf29da211be"
source_paths:
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/worker.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/plugin.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/client/launch.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/client/reader.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/client/mod.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/mod.rs"
session_ids:
  - "67691ee3-3bd2-4e69-9999-7c9150128ad0"
memory_body_ids:
  []
---

# Worker-side `runtime_action` bridge (upstream `rpi-extensions` isolated worker)

Follow-up to the isolated plugin-worker boundary handoff
(`396513a1-8417-44f4-b544-ba0462223293`). That document listed "no
plugin→host runtime-action bridge" as an open gap; this pass closes it inside
the upstream fork's isolated-worker code (no ScribeRig files touched).

All paths below are under
`playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/`.

## Design

- `worker.rs` — new `WorkerHostClient`: worker-side `HostCall` sender coupled to
  host replies over the existing frame protocol, with a shared frame writer,
  reply correlation by id, and a bounded (60 s) reply timeout.
  - A concurrent input-reader thread is started *before* worker
    initialization so `HostResult`/`HostError` frames are serviced while
    plugins are still registering; tool-call frames are dispatched to detached
    threads with per-call cancellation flags.
  - `WorkerHandler::start(Arc<WorkerHostClient>)` lets a handler install the
    bridge before registration (previously a no-op default).
- `plugin.rs` — plugins are loaded through a real `ActionBridge` whose
  `RuntimeActionHost` (`WorkerActionHost`) relays each capability as a
  `WorkerHostClient::call`. Covers `send_message`, session/model/thinking
  actions, `compact`, `reload`, dialogs, child-agent, execution context,
  session branch, and side-question UI. Plugin loading, keepalive, tool
  execution, and the two-thread Tokio action runtime stay inside the worker.
- `client/launch.rs` — `AllowlistedHostHandler`, a fail-closed generic
  host-side capability allowlist wrapper around `WorkerHostHandler`
  (re-exported from `client/mod.rs`).
- `client/reader.rs` — readiness fix: validates `hello` without moving the
  frame early, and routes `HostCall` frames to the host handler while holding
  the write half for replies.

## Rationale / constraints

- Host-action execution is synchronous inside the async bridge call; the
  bounded channel is what keeps a hung host from deadlocking the worker.
- The allowlist is deliberately opt-in and per worker: upstream does not
  impose a default capability set, and argument/operation scoping remains the
  host's responsibility.

## Validation

`rustfmt --check`, `rustfmt`, and `git diff --check` pass on the touched
files. Lengths: `plugin.rs` 397, `client/mod.rs` 315, `worker.rs` 294,
`protocol.rs` 141, `client/launch.rs` 140, `client/reader.rs` 118, `mod.rs` 44.
No builds, tests, clippy, or runtime evidence were produced.

## Remaining gaps

1. Cancellation is not preemptive — the flag is only checked before a call; a
   long-running plugin FFI call cannot be interrupted while it runs.
2. `WorkerHostClient::call` blocks a Tokio worker thread; with only two worker
   threads, concurrent or nested host calls can starve other runtime actions.
3. Allowlist is opt-in; no per-branch/per-tool capability scoping.
4. Lifecycle actions `cancel_side_questions`, `cancel_child_agents`, and
   `replace_model_catalog` remain inert; side-question streaming is not
   supported in workers.
5. No OS sandbox; isolation still depends on a host-supplied `WorkerLauncher`.
6. Registration-time host calls stall until timeout if the host never answers.
7. Upstream pin still not advanced (fork changes uncommitted), so ScribeRig
   cannot compile against the bridge yet.
