# AGENTS.md

## Purpose

Scriberig is a monorepo for a universal semantic knowledge platform and the integrated product built on top of it. Agents working here must preserve the architectural dependency direction described in `design/` and should prefer small, composable changes over cross-layer shortcuts.

## Read design before code

Before changing a subsystem, read the nearest `AGENTS.md` and the relevant `design/level-1`, `design/level-2`, and `design/level-3` documents. Lower-level design documents may refine higher-level documents but must not silently contradict their invariants.

The reusable platform must remain useful without an agent runtime. Product surfaces such as CLI, API server, TUI, web editor, and agents consume shared application capabilities rather than owning duplicated business logic.

## Keep implementation task checklists current

When implementing a `design/level-3/` task, update that task document's
completion checklist as work progresses. Tick each item (`[x]`) as soon as its
acceptance criterion is implemented and verified; keep incomplete or
unverified items unchecked. Before finishing, review every checked item
against the code and validation evidence. Do not defer checklist updates until
the whole task is complete, and do not check items merely because related work
was started.

## Repository map

- `crates/`: reusable Rust libraries and product runtime crates.
- `crates/domains/`: domain adapters such as Markdown, code, Rust, and PDF.
- `apps/`: native executable composition roots.
- `web/packages/`: reusable TypeScript/React libraries.
- `web/apps/`: complete browser applications.
- `tools/`: repository quality, lint, formatting, and developer automation.
- `tests/`: cross-package conformance, protocol, fixtures, and end-to-end tests.
- `design/`: architecture and implementation instructions; Markdown line limits do not apply here.

## File-size rule

Source-code files must contain no more than 400 lines after excluding the leading preamble. The preamble may contain file documentation, license headers, attributes/directives, imports, and `use` declarations. Markdown and other prose/text files are exempt from the 400-line limit.

Do not evade the limit by compressing unrelated logic. Split by responsibility. One preferred split strategy is to replace an oversized file with a directory of the same stem and divide the implementation into focused files. For example:

- Rust: replace `query.rs` with `query/mod.rs`, `query/parse.rs`, `query/execute.rs`, and related focused modules.
- TypeScript: replace `query.ts` with `query/index.ts` plus focused sibling modules.

Keep public re-export boundaries stable when performing a split.

## Declaration documentation and comments

Document each function, method, class, and type in its language's declaration
documentation. Do not add a separate explanatory comment immediately before
that documentation or between the documentation and the declaration. The
repository linter removes and rejects these duplicate descriptions.

Use comments inside implementations only when they explain a non-obvious
decision or constraint that the declaration documentation does not cover.

## File documentation

Every source file must begin with a description docstring/documentation comment before ordinary implementation code, excluding a shebang or license header.

- Rust: use `//!` module/file documentation.
- TypeScript/JavaScript: use a JSDoc file comment containing `@file`.
- Python: use a module docstring.
- Shell/config code that supports comments: start with a short file-purpose comment.

## Function and method documentation

Every function and method, public or private, must be documented.

Documentation must contain:

1. a one-line brief description;
2. an optional extended explanation when the behavior is not obvious;
3. documentation for every parameter;
4. return-value documentation when a meaningful value is returned;
5. exception/error documentation when the function can throw or return an error condition.

Rust convention:

```text
/// Resolve an entity revision in a snapshot.
///
/// # Parameters
/// - `entity`: Entity to resolve.
/// - `snapshot`: Exact snapshot to read.
///
/// # Returns
/// The exact revision when the entity exists.
///
/// # Errors
/// Returns an error when the snapshot cannot be read.
```

TypeScript/JavaScript convention uses JSDoc `@param`, `@returns`, and `@throws` tags as applicable. Python uses `Args:`, `Returns:`, and `Raises:` sections.

## Class and class-like type documentation

Classes must have a one-line brief description and may have an extended description. Their class documentation must contain separate `Public members` and `Private members` sections and document every member in the applicable section. If one section is empty, state that explicitly.

Apply the same intent to class-like constructs in other languages. Rust structs and similar data-bearing types must document their purpose and fields; traits/interfaces must document their contract.

## Automated enforcement

Repository conventions are enforced by `tools/lint_repo.py`, formatters, pre-commit hooks, and CI.

Before committing:

```text
python3 tools/lint_repo.py --fix
cargo fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
pnpm --dir web format
pnpm --dir web typecheck
```

Rust and Clippy warnings are errors. Fix the underlying issue instead of
silencing an actionable warning; any exception must be narrowly scoped and
explained next to the affected code.

Run `tools/install-hooks.sh` once after cloning to install the pre-commit hooks. Hooks intentionally auto-fix safe formatting where possible and reject the commit when a convention still cannot be satisfied automatically.

Never bypass quality hooks to land malformed files. If a rule is producing a false positive, improve the rule or document a narrowly scoped exception in the nearest `AGENTS.md` and lint configuration.

## Dependency direction

The general Rust direction is reusable foundation -> application operations -> adapters/product surfaces. Foundation crates must never depend on agent runtime, server, CLI, TUI, or web concerns.

The web direction is protocol -> client -> model -> editor/react layers -> reusable components -> integrated app. Public packages must never depend on `web/apps/`.

All write surfaces must converge on the shared migration/change model. Do not add direct database mutations from UI, agents, CLI, or transport adapters.

## Change discipline

- Keep scaffolding and architecture-preserving refactors separate from semantic feature implementations where practical.
- Prefer exact immutable IDs and snapshot-bound APIs over hidden mutable-current-state assumptions.
- Do not create duplicate core abstractions because a provisional design used a different crate prefix.
- Every defect fix must add a regression test that reproduces the defect, fails before the fix, and passes after it.
- Add or update tests at the lowest layer that owns a behavior.
- Keep generated files clearly marked and out of hand-edited source paths when possible.

## Temporary agent work

When an agent needs a temporary directory, create it under
`playground/agentic-work/`. Name the directory with the prefix
`YYYMMDD-HHMMSS ` followed by a representative name of no more than 40
characters. Use the directory only for temporary agent work and remove it when
that work is complete.

<!-- BEGIN managed DSH worker instructions -->
## DeepSeek Harness delegation

A local DeepSeek Harness worker is available through the `dsh_worker` MCP server.

Use `dsh_start` for an independent bounded subtask when useful, so you can
continue your own work and later poll `dsh_status`. Use `dsh_run` only when the
result is needed before you can proceed.

Long tasks:

- Every delegated turn may run for up to 2h: `timeout_seconds` defaults to 7200
  (the maximum). The MCP client kills a tool call sooner (Codex: 300s unless
  `tool_timeout_sec` says otherwise), so for anything that may run for minutes,
  prefer `dsh_start` then `dsh_wait(job_id)`.
- `dsh_run` and `dsh_wait` take `wait_seconds` (default 240, max 7200). When the
  slice expires they return `{"status": "running", "job_id": ...}` rather than
  blocking past the client timeout. Keep calling `dsh_wait` on that job id until
  it reports `completed`/`failed`; partial output shows progress in between.
- Treat a returned `job_id` as durable: the work continues in the background even
  if your call returned early.

Session control is yours to choose:

- Start a fresh DSH session by omitting `session_id` (the default).
- Continue an earlier DSH session by passing `session_id=...`, taken from an
  earlier `dsh_start`/`dsh_run` result or from `dsh_sessions(workspace=...)`.
  The worker keeps that session's prior context.
- Use `dsh_sessions` when you need to find prior work for a workspace, then
  decide whether continuing it or starting fresh is cheaper.
- Continue only when the follow-up genuinely builds on that session; start a
  new session when the task is independent, to avoid stale context.
- Keep the same `workspace` when continuing a session; it is scoped to the cwd
  that created it.

Delegation rules:

- `workspace` accepts `"project"`, `"playground"`, `"."`, or any directory
  path. `"."` is the directory this MCP server was started from, normally the
  Codex project, so use it to delegate work in the repository you are in.
- For repository investigation, review, test analysis, locating code, or a
  second opinion, default to `workspace="."` (or `workspace="project"` inside
  scriberig) and `permission_mode="read-only"`.
- For delegated code-writing experiments, prototypes, generated scratch code,
  or changes that should not dirty Git, prefer `workspace="playground"` with
  `permission_mode="workspace-write"`. The playground directory is outside
  source control.
- Use `permission_mode="workspace-write"` against a real repository only when
  a DSH worker truly needs to edit it.
- `permission_mode="danger-full-access"` grants unrestricted machine access.
  It is accepted only for workspaces under the trusted roots (scriberig and
  its playground by default, extended through `DSH_WORKER_FULL_ACCESS_ROOTS`).
  Reserve it for work the user has explicitly authorized and say why you chose
  it. Read-only and workspace-write remain the defaults for ordinary tasks.
- Do not have Codex and DSH edit the same real source files at the same time.
- Give DSH a narrow task with concrete acceptance criteria and useful file
  paths/context. Ask it to report what it inspected/changed and tests it ran.
- Review DSH's result/diff yourself before relying on it or integrating it.
- DSH interactive permission escalation is unavailable; extra reach comes only
  from the `permission_mode` you pass.
- Keep working on tasks that are independent of a running DSH job rather than
  repeatedly polling it immediately.
<!-- END managed DSH worker instructions -->
