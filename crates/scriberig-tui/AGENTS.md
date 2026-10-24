# TUI

`scriberig-tui` presents terminal interaction over shared application capabilities. It must not own storage, query, migration, projection, or agent semantics.

Reusable non-visual terminal-independent behavior belongs below this crate; product composition decides when the TUI is started.

Upstream `rpi-tui` owns the terminal runtime, lifecycle, raw-mode handling, input dispatch, keybindings, themes, generic components, and Crossterm access (Level-2 plan 032 and task 144 §12.2). This crate keeps semantic projections, reducers, and stable ScribeRig action identity only. Do not add a local renderer, terminal owner, event/keymap module, widget framework, or direct `crossterm`/`ratatui` dependency; `crates/scriberig-conformance/tests/upstream_ownership.rs` fails when that boundary moves.
