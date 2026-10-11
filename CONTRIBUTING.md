# Contributing to Scriberig

Read the top-level `AGENTS.md` and the nearest nested `AGENTS.md` before changing code. The design documents under `design/` define architectural intent; implementation must preserve their dependency direction.

## Local setup

```bash
cargo check --workspace --all-targets
corepack enable
pnpm --dir web install
python3 -m pip install pre-commit
./tools/install-hooks.sh
```

## Before opening a change

Run:

```bash
./tools/fix.sh
./tools/check.sh
```

The pre-commit hooks reject source files that violate repository conventions. Do not bypass hooks to land malformed files; fix the code or improve a false-positive rule.

## Source structure

Source code is limited to 400 implementation lines after leading file documentation/import preambles. Markdown and prose files are exempt. Split oversized code by responsibility; a same-stem directory with focused modules is a preferred strategy.

Every source file needs a file-purpose docstring/documentation comment. Every function and method, including private ones, needs parameter/return/error documentation as applicable. Classes need brief/extended documentation and separate public/private member sections. Logical code blocks start after a blank line with a short explanatory `Block:` comment.

## Licensing

Reusable platform components are Apache-2.0 by default. Integrated product/runtime components are AGPL-3.0 by default. Package metadata is authoritative for an individual distributable package. See `LICENSING.md`.

Contributions to a component are made under terms compatible with that component's declared license. This repository does not imply a CLA, DCO, copyright assignment, or commercial relicensing grant. Preserve applicable third-party licenses and notices when adding external material.
