# Scriberig Native Product

This directory is the Rust executable composition root for the integrated product. The Cargo package remains `scriberig`, while the executable it produces is named `srig`. It may depend on product/runtime crates such as `scriberig-server` and `scriberig-agent-runtime` plus reusable Apache-licensed platform crates.

Keep substantive behavior out of this directory. The binary should assemble dependencies and delegate command parsing/mode selection to `scriberig-cli` and runtime behavior to the owning public crates. If logic becomes independently useful, move it into `crates/`.

The native command surface is expected to expose `tui`, `serve`, `run`, `agent`, and `maintenance` modes. A mode may exist first as a CLI stub, but documentation and help text must not imply that unimplemented runtime behavior is already available.
