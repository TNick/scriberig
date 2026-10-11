#!/usr/bin/env bash
# Apply safe repository convention fixes and language formatters.

set -euo pipefail

python3 tools/lint_repo.py --fix || true
cargo fmt --all
pnpm --dir web format
python3 tools/lint_repo.py
