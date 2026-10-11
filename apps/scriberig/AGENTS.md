# Scriberig Native Product

This directory is the Rust executable composition root for the integrated product. It may depend on product/runtime crates such as `scriberig-server` and `scriberig-agent-runtime` plus reusable Apache-licensed platform crates.

Keep substantive behavior out of this directory. The binary should parse startup mode, assemble dependencies, and delegate to public crate APIs. If logic becomes independently useful, move it into `crates/`.
