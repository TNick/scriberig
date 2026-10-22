---
id: "396513a1-8417-44f4-b544-ba0462223293"
title: "Isolated plugin-worker boundary: upstream rpi-extensions worker + ScribeRig host policy adapter"
description: "Implementation handoff for the new versioned isolated upstream plugin-worker IPC boundary and ScribeRig host-policy integration seam, including remaining isolation gaps and validation status."
status: "active"
created_at: "2026-10-21T10:42:28.462Z"
updated_at: "2026-10-21T10:42:28.462Z"
content_hash: "7d3f6fe468f2b7c0c2f4536cfa54d250b2b54d22a9ae5bc98c94082b8ee788f4"
source_paths:
  - "crates/scriberig-agent-runtime/src/plugin_worker.rs"
  - "apps/scriberig/src/native_tui_agent/provider_config/extensions.rs"
  - "apps/scriberig/src/main.rs"
  - "apps/scriberig/src/native_tui_agent.rs"
  - "apps/scriberig/src/main_agent_execution/upstream.rs"
  - "design/level-3/146. replace local provider tool loop and extension host with upstream agent and plugin runtime.md"
  - "design/level-3/149. remove superseded local infrastructure prove delegation and finish the correction.md"
session_ids:
  - "fe4c6493-9b2c-49dc-9570-e90a25fd6e90"
memory_body_ids:
  []
---

# Isolated plugin-worker boundary (implementation handoff)

Date: 2026-10-21. Scope: replace in-process native cdylib plugin loading in the
ScribeRig host with a versioned worker/IPC boundary in the pinned upstream fork,
and add the ScribeRig-side host-policy seam. This addresses the blocker recorded
in task 149 (§§8, 12): `rpi-extensions/src/loader.rs::load_one` loaded plugins via
`libloading::Library::new` inside the host process, so ambient plugin access
could not be constrained to ScribeRig's semantic broker.

## Upstream fork changes (`playground/agentic-work/20261021-204000 pi-tree-virtualization`)

New module `crates/rpi-extensions/src/isolated/`:

- `protocol.rs` — versioned `rpi-plugin-worker-v1` frames: readiness, tool calls,
  results, errors, cancellation, host capability calls, shutdown.
- `client/mod.rs` — worker startup, request correlation, deadlines,
  cancellation, shutdown, and process-tree kill on drop/failure.
- `client/launch.rs` — injected `WorkerLauncher` trait, default
  `HostProcessLauncher`, environment clearing by default, `WorkerLaunchSpec`.
- `client/reader.rs` — response routing, host-capability dispatch,
  connection-failure settlement.
- `worker.rs` — worker-side protocol loop with per-call worker threads.
- `plugin.rs` — loads plugins with `load_session_mixed` only inside the worker
  process; executes registered tools through `PluginToolAdapter`.
- `crates/rpi-extensions/Cargo.toml` — enables the upstream SDK `process`
  feature for process-tree supervision.

File sizes at handoff: mod.rs 41, plugin.rs 165, protocol.rs 141, worker.rs 150,
client/launch.rs 97, client/mod.rs 312, client/reader.rs 117 lines.

Rationale: the boundary, not the loader, becomes the enforcement point. The host
selects the executable/environment and authorizes tool names and semantic
operations; plugin FFI stays in a disposable child process.

## ScribeRig integration

- `crates/scriberig-agent-runtime/src/plugin_worker.rs` (new):
  - `PluginSemanticDispatcher` trait — host-authorized `dispatch(operation,
    arguments)` route that must retain the ScribeRig operation path.
  - `PluginHostPolicy` implementing upstream `WorkerHostHandler`: denies
    unlisted operations *before* reaching the dispatcher (allowlist
    `BTreeSet<String>`).
  - `AuthorizedPluginWorker` — retains the upstream `IsolatedWorker` plus a
    `BTreeSet<String>` of branch-authorized tool ids; starts the worker with a
    cleared environment and host-selected launch spec.
- `apps/scriberig/src/native_tui_agent/provider_config/extensions.rs` — replaces
  `reject_in_process_extensions` blanket rejection with validation of the
  isolated worker configuration; package roots remain rejected.
- `apps/scriberig/src/main.rs:43` — serves `--scriberig-plugin-worker` before CLI
  parsing; worker mode never opens the ScribeRig database.
- `apps/scriberig/src/native_tui_agent.rs:150` and
  `apps/scriberig/src/main_agent_execution/upstream.rs:76` — both entry points
  call `validate_isolated_plugin_configuration()`.
- Workspace `Cargo.toml`, `crates/scriberig-agent-runtime/Cargo.toml`, and
  `Cargo.lock` declare/record `rpi-extensions` (pin still
  `d7db6b3a12278e532a60eefd643949b7db1bbc58`).

Design evidence updated: task 146 (§ "Tool ownership and capability audit",
isolation sequencing note) and task 149 (boundary note ~L197–213, recording that
§§8 and 12 stay unchecked until the worker/IPC API exists *and* the product
consumes it).

## Remaining gaps (do not treat the boundary as finished)

1. **Pin not advanced.** Upstream changes are uncommitted (commit/push was
   forbidden); pin `d7db6b3…` lacks `isolated/`, so ScribeRig will not compile
   against it until the fork revision is committed and pinned.
2. **No production semantic dispatcher.** `PluginSemanticDispatcher` has no
   ScribeRig implementation; worker tools are not merged into the branch
   `ToolCatalog`/`build_tools`, so plugins are still not model-visible.
3. **No OS sandbox.** The default launcher clears env and supervises the process
   tree but does not deny filesystem, network, database, or secret access; a
   host-injected sandbox launcher is still required.
4. **No plugin→host runtime-action bridge.** Worker plugins load without an
   `ActionBridge`, so `runtime_action` calls do not route to host capability RPC.
5. **Cancellation is partial.** Client sends cancellation and the worker records
   it, but long-running plugin FFI does not poll the flag mid-call.
6. **No atomic package/extension-session/catalog generation swap.**

## Validation status

Only `rustfmt --check` on the changed Rust files was run (both the upstream fork
and ScribeRig reported format-OK). No builds, tests, clippy, or runtime isolation
evidence were produced. Task 149 §7–§16 proof items remain unchecked.
