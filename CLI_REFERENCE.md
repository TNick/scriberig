# `srig` CLI reference

This page is the canonical user-facing reference for the `srig` command-line surface. It is published with the rest of the project documentation on GitHub Pages.

Any change that adds, removes, renames, or changes a command, argument, option, default, or CLI-visible behavior must update this page in the same change.

## Synopsis

```text
srig <COMMAND>
srig --help
srig --version
```

The executable requires a subcommand. Standard Clap help and version flags are available:

| Flag | Meaning |
| --- | --- |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print the Scriberig version. |

## Commands

| Command | Synopsis | Status |
| --- | --- | --- |
| `tui` | `srig tui` | Placeholder; terminal UI runtime is not implemented yet. |
| `serve` | `srig serve [OPTIONS]` | Starts the local API server. |
| `run` | `srig run <PROMPT>` | Placeholder; one-shot runtime is not implemented yet. |
| `agent` | `srig agent <PROMPT>` | Placeholder; integrated agent runtime is not implemented yet. |
| `maintenance` | `srig maintenance` | Placeholder; maintenance runtime is not implemented yet. |

Each subcommand also supports `-h` / `--help`.

## `srig tui`

Start the interactive terminal UI.

```text
srig tui
```

The command is currently an honest product stub: it parses successfully and reports that terminal UI mode is not implemented yet.

## `srig serve`

Start the API server.

```text
srig serve [--host <HOST>] [--port <PORT>] [--db <PATH>]
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--host <HOST>` | `127.0.0.1` | Interface address to bind. |
| `--port <PORT>` | `7337` | TCP port to bind. |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database path used by the local API server. |

Example:

```text
srig serve --host 0.0.0.0 --port 7337 --db playground/dev-app-runtime/scriberig.sqlite
```

## `srig run`

Accept one prompt in headless one-shot mode.

```text
srig run <PROMPT>
```

`<PROMPT>` is one required positional string. Quote prompts that contain shell whitespace.

```text
srig run "Summarize the current project state"
```

The command currently accepts the prompt but does not execute a one-shot runtime yet.

## `srig agent`

Accept one prompt for the integrated agent runtime.

```text
srig agent <PROMPT>
```

`<PROMPT>` is one required positional string. Quote prompts that contain shell whitespace.

```text
srig agent "Implement the next focused design task"
```

The command currently accepts the prompt but does not execute the agent runtime yet.

## `srig maintenance`

Select repository or database maintenance mode.

```text
srig maintenance
```

The command currently parses successfully but maintenance operations are not implemented yet.

## Keeping this reference synchronized

The parser in `crates/scriberig-cli` defines the accepted command line, and `apps/scriberig` composes those parsed commands into the executable. Treat this page as part of that public surface: CLI implementation changes are incomplete until this reference describes the resulting command line accurately.
