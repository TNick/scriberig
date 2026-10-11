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

## Design documents

The project is currently being specified from architecture down to implementation tasks under `design/`:

- `design/level-1/` — durable architecture, principles, and system-level decisions;
- `design/level-2/` — concrete crate boundaries, schemas, protocols, algorithms, and package layouts;
- `design/level-3/` — focused implementation instructions and acceptance criteria for implementation agents.

The intended result is a system where humans, agents, compilers, editors, and third-party applications can work with different views of the same persistent semantic state without each inventing a separate storage or mutation model.
