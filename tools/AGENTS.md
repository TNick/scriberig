# Tools

This directory owns repository-wide developer automation. Tooling must enforce repository conventions without owning product semantics.

`lint_repo.py` enforces structural source rules that standard language linters
do not cover. `fix.sh` applies safe automatic repairs and formatters.
`check.sh` runs the non-mutating quality gate used locally and in CI.
`install-hooks.sh` installs the pre-commit configuration.
`precommit_web_format.py` formats only staged web source files with Prettier
through Node and re-stages those exact files, without invoking Bash or WSL.

The repository linter still ignores a `third_party/` path defensively, but the
repository vendors no upstream source tree: the `rpi-*` runtime crates arrive as
pinned Cargo git dependencies, and
`crates/scriberig-conformance/tests/upstream_ownership.rs` fails if a vendored
directory, a `[patch]`/`[replace]` section, a Cargo config override, or an
`rpi-*` workspace member returns.

Tooling may inspect all workspace layers, but product crates must never depend on `tools/`. Keep automation deterministic and conservative: automatic fixes may repair formatting and structure, but must not invent semantic documentation or rewrite behavior.
