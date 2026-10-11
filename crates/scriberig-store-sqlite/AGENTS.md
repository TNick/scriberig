# SQLite Store Backend

`scriberig-store-sqlite` is the first durable embedded implementation of `scriberig-store`. SQLite schema, transactions, recovery metadata, and physical indexes are private implementation details of this crate.

No external consumer should issue SQL against Scriberig's internal tables. Changes to physical storage must preserve backend-independent store behavior and conformance with the memory backend.
