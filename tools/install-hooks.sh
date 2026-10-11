#!/usr/bin/env bash
# Install the repository pre-commit quality gate into the current clone.

set -euo pipefail

if ! command -v pre-commit >/dev/null 2>&1; then
  echo "pre-commit is required. Install it with pipx or pip before continuing." >&2
  exit 1
fi

pre-commit install
pre-commit install-hooks
