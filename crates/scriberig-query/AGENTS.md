# Query Layer

`scriberig-query` owns domain-independent semantic query, traversal, handles, scoping, pagination, and read-operation abstractions over snapshot-bound graph views.

It must not own transport, UI, agent orchestration, or domain-specific parser behavior. Domain extensions may contribute query capabilities through public extension points rather than fork the query engine.
