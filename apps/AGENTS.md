# Native Applications

This directory contains executable composition roots. Applications wire public libraries together; they do not own reusable storage, graph, migration, projection, protocol, or agent abstractions.

`apps/scriberig/` is the integrated native product and may compose the CLI, TUI, server, and agent runtime. Reusable behavior discovered while building the executable must be moved into the appropriate crate under `crates/` rather than left in the binary package.
