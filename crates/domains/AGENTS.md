# Domain Adapters

This directory contains domain-specific schemas and adapters layered over the universal knowledge model.

Domain crates may contribute parsers, renderers, reconcilers, validators, derived indexes, specialized queries, and semantic operations. They must reuse the shared identity, storage, graph, query, migration, derivation, and projection architecture rather than inventing domain-private persistence or agent access.

The initial scaffold reserves adapters for Markdown, language-independent code, Rust syntax/semantics, and PDF import. Domain crates are reusable platform components and therefore remain independent from the integrated agent runtime and product UI.
