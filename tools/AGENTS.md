# Tools

This directory owns repository-wide developer automation. Tooling must enforce repository conventions without owning product semantics.

`lint_repo.py` enforces structural source rules that standard language linters
do not cover. `fix.sh` applies safe automatic repairs and formatters.
`check.sh` runs the non-mutating quality gate used locally and in CI.
`install-hooks.sh` installs the pre-commit configuration.
`precommit_web_format.py` formats only staged web source files with Prettier
through Node and re-stages those exact files, without invoking Bash or WSL.

The repository linter excludes `third_party/` because those files retain their
upstream ownership and formatting. Review vendor changes against `UPSTREAM.md`
and `PATCHES.md` instead of applying Scriberig declaration or file-size rules.

Tooling may inspect all workspace layers, but product crates must never depend on `tools/`. Keep automation deterministic and conservative: automatic fixes may repair formatting and structure, but must not invent semantic documentation or rewrite behavior.
