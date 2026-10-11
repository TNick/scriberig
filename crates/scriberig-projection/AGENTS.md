# Projection Layer

`scriberig-projection` owns domain-independent render/import/reconcile concepts, projection state, and shared filesystem synchronization contracts. Domain adapters provide Markdown/Rust/PDF-specific behavior on top.

Projection writes must become semantic migrations through the shared change model. Do not place domain parsers, UI editors, or agent behavior in this crate.
