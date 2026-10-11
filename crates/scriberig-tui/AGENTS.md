# TUI

`scriberig-tui` presents terminal interaction over shared application capabilities. It must not own storage, query, migration, projection, or agent semantics.

Reusable non-visual terminal-independent behavior belongs below this crate; product composition decides when the TUI is started.
