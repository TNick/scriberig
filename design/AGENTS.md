# Design

This directory contains the architectural source material for implementation agents.

- `level-1/` defines durable system principles and invariants.
- `level-2/` turns those principles into concrete subsystem and package designs.
- `level-3/` contains focused implementation instructions intended to be executable by simpler agents.

Implementation work must read upward: a Level 3 task is constrained by its Level 2 parent and the relevant Level 1 invariants. Design Markdown is exempt from the repository's 400-line source-code limit.
