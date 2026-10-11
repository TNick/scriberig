# Tools

This directory owns repository-wide developer automation. Tooling must enforce repository conventions without owning product semantics.

`repo_lint.py` enforces structural source rules that standard language linters do not cover. `fix.sh` applies safe automatic repairs and formatters. `check.sh` runs the non-mutating quality gate used locally and in CI. `install-hooks.sh` installs the pre-commit configuration. `precommit-web-format.sh` formats only staged web source files with Prettier and re-stages those exact files so formatting fixes can be included in the same commit attempt.

Tooling may inspect all workspace layers, but product crates must never depend on `tools/`. Keep automation deterministic and conservative: automatic fixes may repair formatting and structure, but must not invent semantic documentation or rewrite behavior.
