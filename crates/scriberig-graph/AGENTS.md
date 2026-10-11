# Universal Graph

`scriberig-graph` owns domain-independent semantic revision records, typed relations/collections, descriptions, derived-artifact references, and snapshot-bound graph views.

It is not a database backend and must not hard-code Markdown, Rust, PDF, session, agent, transport, or UI vocabulary. Domain crates define schemas over these universal records.
