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
| `tui` | `srig tui [OPTIONS]` | Starts the local application-backed terminal UI. |
| `serve` | `srig serve [OPTIONS]` | Starts the local API server. |
| `run` | `srig run <PROMPT>` | Runs the deterministic local one-shot provider. |
| `agent` | `srig agent <PROMPT>` | Runs the deterministic local agent runtime. |
| `maintenance` | `srig maintenance` | Checks SQLite canonical-state integrity. |
| `inspect` | `srig inspect <ENTITY> [OPTIONS]` | Inspects entity trust metadata through the shared application boundary. |

Each subcommand also supports `-h` / `--help`.

## `srig tui`

Start the local application-backed terminal UI.

```text
srig tui [--db <PATH>]
```

The UI reads semantic data through the shared application boundary. Type
`inspect <ENTITY>` to inspect an entity, or `q` to quit.

| Option | Default | Meaning |
| --- | --- | --- |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database path used by the local TUI composition. |

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

The local runtime echoes the prompt through the provider boundary until a
configured model provider is composed.

## `srig agent`

Accept one prompt for the integrated agent runtime.

```text
srig agent <PROMPT>
```

`<PROMPT>` is one required positional string. Quote prompts that contain shell whitespace.

```text
srig agent "Implement the next focused design task"
```

The local runtime executes the prompt through the bounded agent runtime and
provider/tool boundaries.

## `srig maintenance`

Select repository or database maintenance mode.

```text
srig maintenance
```

The command runs a canonical SQLite integrity check against
`playground/dev-app-runtime/scriberig.sqlite` and reports corruption and
unreachable snapshot counts.

## `srig inspect`

Inspect trust metadata for one semantic entity.

```text
srig inspect <ENTITY> [OPTIONS]
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--snapshot <SNAPSHOT>` | current | Inspect an exact snapshot when supplied. |
| `--depth <compact\|deep>` | `compact` | Include provenance and evidence handles at `deep`. |
| `--format <human\|json>` | `human` | Select terminal text or structured JSON output. |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database used by local inspection. |

The command opens the configured SQLite store and reads immutable revisions and
snapshots through the shared application boundary. A missing entity, snapshot,
or current `main` root is reported as a structured failure. Authorization
denials use exit status `77`.

## Keeping this reference synchronized

The parser in `crates/scriberig-cli` defines the accepted command line, and `apps/scriberig` composes those parsed commands into the executable. Treat this page as part of that public surface: CLI implementation changes are incomplete until this reference describes the resulting command line accurately.
