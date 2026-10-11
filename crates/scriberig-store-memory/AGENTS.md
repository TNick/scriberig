# Memory Store Backend

`scriberig-store-memory` is an in-memory implementation of `scriberig-store`. It serves as an executable specification, fast test backend, and embedding option for callers that do not need durable persistence.

It must pass the same storage conformance suite as durable backends and must not expose semantics unavailable through the backend-independent store contracts.
