# Core Types

`scriberig-core` is the bottom reusable Rust layer. It owns stable identifiers and small domain-neutral value/provenance types that must not depend on storage backends, domain adapters, agents, transports, or UI.

Keep this crate deliberately small. If a concept requires graph traversal, persistence, application operations, or one domain's semantics, it belongs above core.
