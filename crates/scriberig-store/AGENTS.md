# Store

`scriberig-store` owns backend-independent persistence contracts for immutable revisions, snapshots, named roots, reachability, integrity, and history primitives. It must not expose SQLite or filesystem implementation details.

Concrete backends live in sibling crates such as `scriberig-store-memory` and `scriberig-store-sqlite` and depend on this crate, never the reverse. Higher semantic layers consume these contracts without issuing backend-specific operations.
