# Rust Crates

This directory contains Rust libraries arranged from reusable foundation to integrated product runtime.

Dependency direction is one-way: core/store/schema -> graph/query/change/derive/projection/session -> application/protocol/client -> agent/product adapters. Foundation crates must never depend on CLI, TUI, server, agent runtime, or web concerns.

Storage backends implement `scriberig-store` contracts. Domain adapters live under `crates/domains/`. `scriberig-app` is the shared application-operation boundary used by CLI, server, TUI, web transport, and agents. `scriberig-agent-runtime` and `scriberig-server` are integrated product/runtime layers and must not leak product dependencies back into reusable crates.

When adding a crate, document its layer, allowed dependencies, and license in its manifest. Prefer extending an existing architectural owner over creating a competing crate for the same concept.
