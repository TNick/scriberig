#!/usr/bin/env bash
# Format staged web source files with Prettier and re-stage the exact files.

set -euo pipefail

repo_files=()
web_files=()

for path in "$@"; do
    case "$path" in
        web/*.ts|web/*.tsx|web/*.js|web/*.jsx)
            repo_files+=("$path")
            web_files+=("${path#web/}")
            ;;
    esac
done

if (( ${#repo_files[@]} == 0 )); then
    exit 0
fi

(
    cd web
    pnpm exec prettier --write "${web_files[@]}"
)

git add -- "${repo_files[@]}"
