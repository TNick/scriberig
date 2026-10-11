#!/usr/bin/env bash
# Run the repository's non-mutating local quality gate.

set -euo pipefail

python3 tools/lint_repo.py
python3 tools/check_persistence_boundaries.py
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
pnpm --dir web format:check
pnpm --dir web typecheck
