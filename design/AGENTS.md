# Design

This directory contains the architectural source material for implementation agents.

- `level-1/` defines durable system principles and invariants.
- `level-2/` turns those principles into concrete subsystem and package designs.
- `level-3/` contains focused implementation instructions intended to be executable by simpler agents.

Implementation work must read upward: a Level 3 task is constrained by its Level 2 parent and the relevant Level 1 invariants. Design Markdown is exempt from the repository's 400-line source-code limit.

While implementing a Level 3 task, keep its completion checklist in sync with
the work: check off each acceptance criterion when it is implemented and
verified, leave incomplete or unverified criteria unchecked, and review all
checked items against the final code and validation evidence before finishing.

The design filenames were renumbered to preserve unique numeric prefixes after some documents had already been written. When an older document contains a stale numbered path, identify the referenced document by title and use its current filename rather than recreating the obsolete numbering.

For repository scaffold work, consult `level-2/020. repository scaffold conventions and native command surface.md` and `level-3/098. reconcile scaffold task status and supersession.md`. The reconciliation document identifies overlapping historical scaffold tasks that are Partial or Superseded and must not be treated as current implementation authority.

For agent prompt/profile/context-composition work, read `level-1/021. agent profiles context composition and evidence driven optimization.md`, `level-2/030. agent profile context component and prompt experiment runtime.md`, and `level-3/137. implement agent profiles branch handoff context components and prompt experiments.md`. These refine earlier branch/runtime wording: a child branch is a context-isolation frame and does not by itself imply a new agent/executor identity.

For personalized source projection styles, repository/materialization policy, or lossy editable projections, read `level-1/024. projection styles personalized representations and materialization policy.md`, `level-2/034. projection style resolution loss-aware reconciliation and cross-surface contracts.md`, and Level-3 tasks 151 through 153. These refine the generic projection contracts in Level-1 006 and Level-2 007/008. Terminal integration must follow the current upstream-first ownership boundary in Level-2 032: upstream `rpi-tui` owns commodity terminal runtime/theme/control infrastructure, while Scriberig owns semantic projection-style resolution, fidelity, materialization, and reconciliation semantics.
