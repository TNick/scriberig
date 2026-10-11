# CLI

`scriberig-cli` owns command parsing and dispatch over public application/client capabilities. It is a reusable Apache-licensed surface and must not become an alternate home for business logic.

The native executable is `srig`. Its stable top-level mode names are `tui`, `serve`, `run`, `agent`, and `maintenance`, with standard help and version output. Modes may be introduced as honest stubs before their owning runtime is implemented.

Commands delegate to `scriberig-app`, `scriberig-client`, or explicitly composed product modes. Keep persistence, semantic mutation, server behavior, terminal rendering, and agent orchestration in their owning crates. Command parsing may describe and validate arguments, but it must not duplicate those implementations.

## CLI reference is part of the public surface

`CLI_REFERENCE.md` is the canonical user-facing reference for the full `srig` command surface and is published on GitHub Pages.

Whenever a change adds, removes, renames, or changes a command, subcommand, positional argument, option, alias, default value, help-visible behavior, or other CLI-visible behavior, update `CLI_REFERENCE.md` in the same change. A CLI change is incomplete if the reference page no longer matches the parser and executable behavior.
