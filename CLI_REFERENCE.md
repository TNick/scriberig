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
| `run` | `srig run <PROMPT> [OPTIONS]` | Runs one shared application agent operation. |
| `agent` | `srig agent <PROMPT> [OPTIONS]` | Alias for the shared application agent operation. |
| `maintenance` | `srig maintenance [OPTIONS]` | Checks SQLite canonical-state integrity. |
| `inspect` | `srig inspect <ENTITY> [OPTIONS]` | Inspects entity trust metadata through the shared application boundary. |
| `capabilities` | `srig capabilities [OPTIONS]` | Lists capabilities exposed by a remote application. |
| `operation` | `srig operation <status\|cancel> <OPERATION> [OPTIONS]` | Observes or cancels a remote operation. |
| `events` | `srig events [OPTIONS]` | Reads buffered lifecycle or agent output events. |
| `session` | `srig session <tree\|activity> ... [OPTIONS]` | Reads durable session and branch state. |
| `agent-continue` | `srig agent-continue <SESSION> <BRANCH> <PROMPT> [OPTIONS]` | Continues an agent operation on an existing branch. |
| `config` | `srig config show [--format <human\|json>]` | Prints effective local-agent settings and their sources. |
| `secrets` | `srig secrets <set\|list\|status\|delete\|bind\|unbind> ...` | Manages local OS credentials and exact destination bindings. |
| `doctor` | `srig doctor [--format <human\|json>]` | Sends a bounded provider request using the effective credential policy. |
| `recover-profile` | `srig recover-profile <SESSION> --revision <N> --profile <JSON> --provider <ID> --model <ID> [OPTIONS]` | Migrates one preserved legacy profile selection from an explicit operator template. |

Each subcommand also supports `-h` / `--help`.

## `srig tui`

Start the local application-backed terminal UI.

```text
srig tui [--debug] [--log-file <PATH>] [--db <PATH> | --connect <URL>]
```

The UI reads semantic data through the shared application boundary. Type
`inspect <ENTITY>` to inspect an entity, or `q` to quit.

| Option | Default | Meaning |
| --- | --- | --- |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database path used by the local TUI composition. |
| `--connect <URL>` | none | Public protocol endpoint used by the remote TUI composition. Cannot be combined with `--db`. |
| `--debug` | disabled | Write content-redacted runtime diagnostics to `playground/tui-logs/YYYYMMDD-HHMMSS.log`. |
| `--log-file <PATH>` | generated timestamped path | Override the debug log destination. Requires `--debug`. |

## `srig serve`

Start the API server.

```text
srig serve [--host <HOST>] [--port <PORT>] [--db <PATH>] [--allow-origin <ORIGIN>]
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--host <HOST>` | `127.0.0.1` | Interface address to bind. |
| `--port <PORT>` | `7337` | TCP port to bind. |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database path used by the local API server. |
| `--allow-origin <ORIGIN>` | unset | Exact browser origin allowed for cross-origin protocol requests. |

Example:

```text
srig serve --host 0.0.0.0 --port 7337 --db playground/dev-app-runtime/scriberig.sqlite
```

## `srig run`

Accept one prompt in headless one-shot mode.

```text
srig run <PROMPT> [OPTIONS]
```

`<PROMPT>` is one required positional string. Quote prompts that contain shell whitespace.

```text
srig run "Summarize the current project state"
```

The command creates a fresh session root, submits the prompt through the shared
application operation boundary, waits for the terminal operation record, and
renders the structured agent result. Human output writes final text to stdout
and stable handles to stderr. When a streaming provider is configured, bounded
transient model text is also reported on stderr as it arrives; the durable
final answer remains the only answer text written to stdout. JSON output writes
one machine-readable result and does not emit transient stream text.

| Option | Default | Meaning |
| --- | --- | --- |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database path used by the local composition. |
| `--format <human\|json>` | `human` | Select human or machine-readable output. |
| `--no-stream` | disabled | Disable transient provider output without changing operation semantics. |
| `--provider <PROFILE>` | `default` | Safe provider profile identifier. |
| `--model <MODEL>` | `default` | Provider model identifier when using `--base-url`. |
| `--base-url <URL>` | unset | OpenAI-compatible provider base URL. When omitted, deterministic local execution is used. |
| `--api-key-env <NAME>` | unset | Compatibility environment credential reference; conflicts with `--credential-ref`. |
| `--credential-ref <REFERENCE>` | unset | `keyring:NAME`, `environment:NAME`, `file:ABSOLUTE_PATH`, or `none` to clear an inherited credential. |

`run` and `agent` resolve each setting independently in this order: system
configuration file, user configuration file, nearest project configuration
file, matching `SRIG_*` environment variable, explicitly supplied option, then
the built-in default. Files are named `config.yaml`, `config.yml`, or
`config.json` and contain a flat object of setting names such as `provider`,
`model`, `base_url`, and `no_stream`.

On Windows, system and user files are searched under `%ProgramData%\Scriberig`
and `%APPDATA%\Scriberig`. On macOS they are searched under
`/etc/scriberig` and `~/Library/Application Support/Scriberig`. On other Unix
systems they are searched under `/etc/scriberig` and `$XDG_CONFIG_HOME/scriberig`
(or `~/.config/scriberig` when `XDG_CONFIG_HOME` is unset). Project discovery
walks upward from the working directory. At each level, YAML is preferred over
YML, which is preferred over JSON.

Environment names use the `SRIG_` prefix and uppercase property name, for
example `SRIG_PROVIDER`, `SRIG_BASE_URL`, and `SRIG_NO_STREAM`. Credential
values remain indirect: `api_key_env` names the environment variable to read
for the provider request.

Inspect effective values and their winning source with:

```text
srig config show [--format human|json]
```

The command reports each supported setting, its value, and a source path or
designator such as `env:SRIG_MODEL`, `cli:--model`, or `default`. The credential
environment variable name may appear in the atomic `credential` reference. The named
secret variable is not read or printed by this command.

Credential references require an explicit destination/project binding before
the value is retrieved. An explicit local CLI environment reference paired with
an explicit `--base-url` supplies a binding for that invocation only, subject to
administrator restrictions. Project configuration cannot authorize new uses.
URLs cannot contain embedded credentials, queries, or fragments.

Pressing Ctrl-C requests cancellation of the shared operation. The command
preserves the operation/session record and exits with status `130`.

## `srig secrets`

```text
srig secrets set NAME [--stdin] [--replace]
srig secrets list [--format human|json]
srig secrets status NAME [--format human|json]
srig secrets delete NAME
srig secrets bind NAME --base-url URL --project-root PATH [--service]
srig secrets unbind NAME --base-url URL --project-root PATH [--service]
```

`set` reads a hidden terminal prompt or explicitly selected bounded stdin. It
never accepts a value argument. `list` reads secret-free registry metadata;
`status` checks native-store metadata without a provider request. Binding names
may be managed keyring aliases or typed environment/file references. Management
is local CLI only. See [Managing secrets](SECRETS.md) for storage locations,
deployment guidance, stable error statuses, rotation, and exposure recovery.

## `srig recover-profile`

Migrate a legacy `/execution` profile selection using a complete operator
authored semantic profile and an explicit current upstream model selection:

```text
srig recover-profile <SESSION> --inspect [--db <PATH>]
srig recover-profile <SESSION> --revision <N> --profile <JSON> --provider <ID> --model <ID> [--thinking-level <ID>] [--db <PATH>] [--yes]
```

Run `--inspect` first to read the exact preserved profile identity and current
execution-selection revision for the session. Then author a complete profile
template and run the apply form with an explicit current upstream selection.
The JSON file must contain an `AgentProfileDescriptor`, including the exact
legacy profile identity in `reference.identity`, the profile name, and its
complete context-component policy. The command does not infer profile content
or provider/model identity from the retired profile name, history, or catalog.
It validates the requested model against the active upstream catalog, prints
the candidate identity and revision, and prompts before promoting it. After
promotion, it resolves the matching legacy session record using the supplied
selection revision. If the revision is stale or the record is absent, the
command reports the failure and leaves the recovery record available for
review.

| Argument | Meaning |
| --- | --- |
| `<SESSION>` | Exact `ent_` session identifier containing the legacy record. |
| `--inspect` | Show the legacy identity, preserved selection, and observed revision. |
| `--revision <N>` | Execution-selection revision observed during recovery inspection. |
| `--profile <JSON>` | Operator-authored complete semantic profile descriptor. |
| `--provider <ID>` | Current upstream provider identifier. |
| `--model <ID>` | Current upstream provider-scoped model identifier. |
| `--thinking-level <ID>` | Optional advertised thinking-level identifier. |
| `--db <PATH>` | SQLite database; defaults to `playground/dev-app-runtime/scriberig.sqlite`. |
| `--yes` | Skip interactive confirmation. |

## `srig doctor`

Check the effective provider connection and credential:

```text
srig doctor [--format human|json]
```

Doctor resolves layered provider settings from the same system, user, project,
and environment sources as `run`, checks saved credential authorization for
the current project and endpoint, then asks the configured model to end its
response with `MODEL OK`. Brief reasoning may precede that marker. The request
is bounded to 512 output tokens; doctor does not print the response or
credential. The request may use provider quota. A failed check exits with
status `1`; JSON output includes the provider, model, endpoint, credential
status, API status, and safe error detail.

## `srig agent`

Accept one prompt for the integrated agent runtime.

```text
srig agent <PROMPT> [OPTIONS]
```

`<PROMPT>` is one required positional string. Quote prompts that contain shell whitespace.

```text
srig agent "Implement the next focused design task"
```

`agent` delegates to exactly the same shared application operation path as
`run`; the two spellings are retained for compatibility.

## `srig maintenance`

Select repository or database maintenance mode.

```text
srig maintenance [--db <PATH>]
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--db <PATH>` | `playground/dev-app-runtime/scriberig.sqlite` | SQLite database to check. |

The command runs a canonical SQLite integrity check against the selected
database and reports corruption and unreachable snapshot counts.

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

## Remote operation commands

The following commands use the public protocol client. They default to
`http://127.0.0.1:7337` and accept `--connect <URL>` for another server.

```text
srig capabilities [--connect <URL>] [--json]
srig operation status <OPERATION> [--connect <URL>]
srig operation cancel <OPERATION> [--connect <URL>]
srig events [--kind <EVENT_KIND>]... [--after <CURSOR>] [--connect <URL>]
srig session tree <SESSION> [--connect <URL>]
srig session activity <SESSION> <BRANCH> [--connect <URL>]
srig agent-continue <SESSION> <BRANCH> <PROMPT> [--connect <URL>]
```

`events --kind agent_output_delta` observes bounded streaming output. Events
are returned as structured JSON so callers can retain the cursor and recover
durable state through `operation` and `session` commands.

## Keeping this reference synchronized

The parser in `crates/scriberig-cli` defines the accepted command line, and `apps/scriberig` composes those parsed commands into the executable. Treat this page as part of that public surface: CLI implementation changes are incomplete until this reference describes the resulting command line accurately.
