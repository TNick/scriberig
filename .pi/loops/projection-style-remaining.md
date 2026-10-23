# Projection style: remaining level-3 slice (tasks 1-3)

Repo: `/root/repos/ai/scriberig-level-3` (branch `feat/level-3-remaining`).
Always build with `CARGO_TARGET_DIR=/root/repos/ai/scriberig-level-3/target` — the
shared target dir of the sibling checkout serves stale `sr-projection` artifacts
and produces phantom "unresolved import" errors.
Design context: `design/level-3/152. implement style-aware rust and ecma source
projection rendering.md` (§5b lists the exact seams). Do not open `target/`.

Work one task at a time. A task is done only when it compiles, its tests pass,
`cargo fmt --all --check` and `python3 tools/lint_repo.py` are clean, clippy is
clean for the touched packages (`-- -D warnings`), and the change is committed
with a message naming what it enables and the commands run. Every new public
item needs full rustdoc (params, `# Returns`, `# Errors`, `# Panics` where
applicable); files stay under 400 implementation lines, so split rather than
grow.

## Tasks

- [x] **1. Durable projection style settings store.** Implement a SQLite-backed
      `ProjectionStyleStore` (scope, scope identity, projection kind, canonical
      style bytes, monotonic revision with optimistic compare-and-swap and clear)
      in a crate that already owns durable state (`scriberig-store-sqlite` or a
      small crate under `crates/`), installed at the composition sites
      (`apps/scriberig/src/projection_style_composition.rs`) instead of
      `InMemoryProjectionStyleStore`, with the store path derived from the
      existing database path. Requirements: a preference written by one process
      is readable after reopening the database; a stale `expected_revision`
      yields `ApplicationError::SemanticConflict`; settings writes still touch no
      canonical semantic state (assert the recording-backend probe stays at 0).
      Extend `crates/scriberig-app/tests/projection_style_settings.rs` (or a new
      test file reusing `projection_style_fixture.rs`) rather than growing any
      file past the limit. Update the task-152 checklist item to Done with the
      evidence and commands.

- [ ] **2. Style selection on the render operation.** Add an explicit style
      selection to `scriberig_app::RenderRequest` (currently `{ snapshot, view }`,
      `crates/scriberig-app/src/request.rs`) as a variant/enum with at least:
      use the resolved effective style, exact inline `StyleValue`, or explicit
      personal/export selection — no ambient borrowing. Resolve it where the
      application has style-configuration access (not in the provider-side
      `resolve_render` helpers of `operation_backend.rs` /
      `composite_backend.rs`, which cannot see
      `ApplicationService::projection_style`): resolve with
      `resolve_effective_style` using the caller's purpose (`InteractiveView` for
      view renders, `SharedMaterialization` for materializing renders) and pass
      the exact `ResolvedStyle` into `sr_projection::RenderRequest`, which already
      requires it. Mirror the field through `scriberig-protocol` (request/result
      contract and exported schema) and the TUI callers
      (`crates/scriberig-tui/src/view/projection.rs`,
      `crates/scriberig-tui/src/state_reducer/run.rs`,
      `crates/scriberig-tui/src/service_project_projection.rs`). Prove with tests:
      the same semantic target renders differently under two explicit styles; a
      shared-materialization render never picks up a stored personal preference;
      an unregistered kind fails with a structured unsupported error rather than a
      generic style. Keep existing protocol tests green; update
      `design/level-3/152. …md` checklist.

- [ ] **3. Materialization style in the filesystem projection path.** In
      `crates/sr-projection-fs/src/import.rs` (render requests around lines 68 and
      218), take the exact `ResolvedStyle` from the caller instead of any local
      default, and at the product call site resolve it once with purpose
      `SharedMaterialization` (workspace materialization policy applies; personal
      user preference and view override must not). Requirements: two different
      workspace materialization styles produce different materialized file bytes
      for one semantic target; the materialized output never reflects a stored
      personal preference; render cache identity tracks the style (a style change
      must not be served from a stale cache entry). Update the task-152 checklist
      item with evidence.

## Not in scope here

ECMA rendering (that domain has only a parser — needs its own renderer task) and
task 153 (loss-aware reconciliation, shared TUI/web style controls).
