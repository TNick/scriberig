# Design

This directory contains the architectural source material for implementation agents.

- `level-1/` defines durable system principles and invariants.
- `level-2/` turns those principles into concrete subsystem and package designs.
- `level-3/` contains focused implementation instructions intended to be executable by simpler agents.

Implementation work must read upward: a Level 3 task is constrained by its Level 2 parent and the relevant Level 1 invariants. Design Markdown is exempt from the repository's 400-line source-code limit.

The design filenames were renumbered to preserve unique numeric prefixes after some documents had already been written. When an older document contains a stale numbered path, identify the referenced document by title and use its current filename rather than recreating the obsolete numbering.

For repository scaffold work, consult `level-2/020. repository scaffold conventions and native command surface.md` and `level-3/098. reconcile scaffold task status and supersession.md`. The reconciliation document identifies overlapping historical scaffold tasks that are Partial or Superseded and must not be treated as current implementation authority.
