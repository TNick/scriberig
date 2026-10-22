---
id: "70cd80f1-35d4-46fd-ad5c-c95859f358a7"
title: "Isolated plugin-worker cancellation and shutdown hardening (upstream rpi-extensions fork)"
description: "Implementation handoff for cooperative cancellation tokens, bounded thread settlement, and hard host-deadline process-tree termination in the isolated plugin worker, with the explicit FFI preemption limits."
status: "active"
created_at: "2026-10-21T11:19:58.039Z"
updated_at: "2026-10-21T11:19:58.039Z"
content_hash: "6c961a925e4c680ec21326ebb14e755a2165b10e883f3d92372ac033e0f215b2"
source_paths:
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/worker.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/plugin.rs"
  - "playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/client/mod.rs"
session_ids:
  - "76162fc9-117f-407b-b001-b30b0657bdfb"
memory_body_ids:
  []
---

# Isolated plugin-worker cancellation and shutdown hardening

Date: 2026-10-21. Follow-up to the isolated plugin-worker boundary handoff
(`396513a1-8417-44f4-b544-ba0462223293`) and the worker-side `runtime_action`
bridge handoff (`73c15a88-1530-449c-8a28-9758453c4274`). Scope was limited to
the controlled upstream fork's isolated worker lifecycle; ScribeRig activation,
the sandbox launcher files/docs, and `rpi-ai`/`rpi-agent` provider-error work
were deliberately untouched.

All paths below are under
`playground/agentic-work/20261021-204000 pi-tree-virtualization/crates/rpi-extensions/src/isolated/`.

## Changes

- `worker.rs` — replaced the per-call `AtomicBool` with a `CancellationToken`.
  The main loop tracks spawned tool threads in a shared handle vector (pruning
  finished handles on each dispatch), applies cancellation to every active call
  on `Shutdown`/stream close, then waits up to a 2 s grace period via
  `settle_tool_threads`, joining threads that finished. The input-reader thread
  is joined after the main loop ends. If handles remain active after the grace
  period, a `tracing::warn!` records that cooperative stop failed and process
  exit is required.
- `plugin.rs` — the real cancellation token is passed into
  `PluginToolAdapter::execute`, enabling the upstream plugin driver's
  cooperative cancel callback; cancellation is checked before and after the
  plugin call.
- `client/mod.rs` — a host request deadline is now a hard failure: the worker
  process tree is terminated, the connection is marked closed, and outstanding
  requests fail instead of leaving plugin work running.

## Cancellation guarantees the code actually provides

- A host `Cancel` frame cancels the worker token; the plugin adapter receives it
  and may invoke the plugin's cooperative cancellation hook.
- Cancellation is checked before starting a call and after the adapter returns.
- `Shutdown` or input-stream closure cancels all active tool calls and gives
  them up to 2 s to settle; finished threads are joined.
- The reader thread is joined after the main loop exits.
- Host request timeout is a hard failure: process tree killed, pending requests
  errored.

## Remaining limits

- **Native plugin FFI cannot be safely preempted.** A plugin that ignores its
  cancellation hook and stays inside native code keeps its tool thread alive;
  after the grace period the thread is detached and only worker process exit
  terminates it. This is the core reason the worker must be a disposable
  process.
- **Cancellation is not acknowledged.** The host sends `Cancel` and immediately
  returns `Cancelled`; it does not wait for confirmation that plugin execution
  stopped.
- **Host capability calls are not cancellation-aware.** A tool blocked in
  `WorkerHostClient::call` can still wait up to its 60 s reply timeout after the
  tool token is cancelled.
- **Plugin registration cannot be cancelled.** A `Shutdown` frame arriving while
  `handler.start` loads/registers plugins is queued until registration returns.
- **Shutdown grace mismatch.** The worker allows 2 s for cooperative
  settlement while the host shutdown path waits roughly 500 ms before killing
  the tree; the host may terminate before cooperative cancellation finishes.
  This is a deliberate process-level fallback, not graceful completion.
- **In-process embedding caveat.** Detached unfinished tool threads are only
  safe because the worker binary exits; a host embedding `serve_worker` in a
  long-lived process would retain them.

## Validation

`rustfmt --check` passes on `worker.rs`, `plugin.rs`, and `client/mod.rs`;
scoped `git diff --check` passes for tracked files. No builds, tests, clippy, or
runtime verification were run — the behavioral guarantees above are
code-reading claims pending runtime evidence.
