# Change/Migration Layer

`scriberig-migration` is the canonical domain-independent change layer for migrations, semantic diffs, history topology, branches/workspaces, merge/conflict handling, and lineage. All mutation surfaces converge here or through application operations built on it.

It depends on reusable core/graph/store abstractions and must not depend on domain adapters, agents, CLI, HTTP, TUI, or web UI.
