# Scriberig

Scriberig is a structured knowledge and agent platform for working with code, documents, PDFs, sessions, project decisions, tool results, and other domains through one common model.

Its central idea is:

> Store meaning as structured knowledge. Treat files, transcripts, user interfaces, and model context as projections of that knowledge.

Instead of making files or chat transcripts the primary source of truth, Scriberig models knowledge as typed entities, relations, immutable revisions, and snapshots. Supported files remain normal human-editable projections: changes made in external editors can be parsed and reconciled back into semantic changes.

The same foundation is intended to support ordinary applications as well as agents. Storage, querying, domain adapters, projections, CLI, API, TUI, reusable web libraries, and the full editor are designed as layers over shared capabilities. Agents are consumers of those capabilities rather than privileged owners of the system.

Key goals include:

- structured, progressively disclosed access instead of requiring agents to read large raw files;
- immutable revisions, snapshots, branches, semantic diffs, and explicit migrations;
- local incremental updates and reusable derived results;
- bidirectional projections for source files and documents;
- one interaction model across code, documents, PDFs, sessions, and future domains;
- reusable Rust and web libraries that remain useful without the agent runtime;
- CLI, API, TUI, web, and agent surfaces that evolve together.

## Repository layout

```text
crates/        Rust libraries and runtime crates
apps/          Native executable composition roots
web/packages/  Reusable TypeScript/React packages
web/apps/      Complete browser applications
tools/         Repository linting and developer automation
tests/         Cross-layer conformance and end-to-end tests
design/        Level 1-3 architecture and implementation documents
```

## Design documents

The project is currently being specified from architecture down to implementation tasks under `design/`:

- `design/level-1/` — durable architecture, principles, and system-level decisions;
- `design/level-2/` — concrete crate boundaries, schemas, protocols, algorithms, and package layouts;
- `design/level-3/` — focused implementation instructions and acceptance criteria for implementation agents.

The intended result is a system where humans, agents, compilers, editors, and third-party applications can work with different views of the same persistent semantic state without each inventing a separate storage or mutation model.

## Documentation

Provider credentials and their destination bindings are described in
[Managing secrets](SECRETS.md). Use `srig config show` to inspect references and
configuration provenance without retrieving credential values.

The GitHub Pages site publishes the project documentation together with generated API references:

- [CLI reference](https://tnick.github.io/scriberig/cli-reference/)
- [Rust and TypeScript API reference](https://tnick.github.io/scriberig/api/)
- [TypeScript API documentation](https://tnick.github.io/scriberig/api/typescript/)

The API documentation is rebuilt from the Rust workspace and public TypeScript packages by the GitHub Pages workflow.

## Bootstrap

```bash
rustup show
cargo check --workspace --all-targets
corepack enable
pnpm --dir web install
python3 -m pip install pre-commit
./tools/install-hooks.sh
```

## Quality checks

Run the complete local quality gate with:

```bash
./tools/check.sh
```

Apply safe automatic fixes and formatters with:

```bash
./tools/fix.sh
```

## Build and install

## Project onboarding

Projects are stored in the selected SQLite database and multiple projects may
share that database:

```text
srig project scan PATH --db scriberig.sqlite
srig project scan PATH --git-tracked --db scriberig.sqlite
srig project list --db scriberig.sqlite
srig project import PROJECT_ID --db scriberig.sqlite
srig project status PROJECT_ID --db scriberig.sqlite
```

To onboard this repository with one command, import the files tracked by Git,
and print the resulting status:

```text
make onboard
```

Set `ONBOARD_DB` to use a shared database other than the development database:

```text
make onboard ONBOARD_DB=path/to/shared.sqlite
```

The API server can onboard only paths under explicitly configured roots:

`--git-tracked` restricts the persisted scan manifest to existing files in
Git's index. Import then consumes that reviewed manifest.

Source imports currently include Markdown, Rust, PDF, HTML, JavaScript,
TypeScript, Python (.py, .pyi, and .pyw), C (.c and .h), and C++ (.cc, .cpp,
.cxx, .hpp, .hh, and .hxx) files. JSON/JSONC, YAML, and TOML retain parsed
value trees. Common project, configuration, documentation, script, and source
files retain deterministic text-line entities. Binary artifacts retain exact
source bytes plus media-type and size metadata. Unknown UTF-8 files use the
same text adapter, so no file is silently dropped from onboarding.

```text
srig serve --db scriberig.sqlite --project-root PATH
```

The TUI can scan a project before startup and optionally import it:

```text
srig tui --db scriberig.sqlite --project PATH --import
```

Build all Rust workspace targets with:

```text
make build
```

Build and install the `srig` executable into Cargo's bin directory with:

```text
make install
```

## Local app

Start or reuse the API and web application, create the ignored SQLite database
under `playground/dev-app-runtime/`, and open the editor in your browser with:

```text
make dev
```

Stop the processes started by that command with:

```text
make dev-stop
```

The Make targets work on Linux and Windows. Repeating `make dev` reuses the
running services; `make dev-stop` stops them. The current API exposes health and
semantic operation endpoints. Optional domain operations remain
capability-gated; agent streaming is available through the runtime and
supported product surfaces. Logs and the local database stay in
`playground/dev-app-runtime/`.

## Product workflow status

The repository contains a deterministic Rust project golden fixture and
real-process browser, native TUI, and cross-surface acceptance suites. These
currently exercise project activation, named `Calculator`/`add` navigation,
Rust source projection, reviewed candidate validation, conflict-protected
materialization, reload durability, and semantically scoped deterministic
agent candidates. The self-hosting gates also cover validation failure, safe
onboarding cancellation, and stale-base rejection. Run
`python tools/product_golden.py` to execute the complete acceptance driver.
The implementation is not described as product-verified
until the design checklists also confirm durable candidate/effect recovery and
public embedded/remote conformance evidence.

Repository-specific source and agent rules live in the top-level `AGENTS.md` and are refined by nested `AGENTS.md` files.
