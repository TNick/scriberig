# SQLite migrations

Storage migrations in this directory are private to the SQLite backend. They must preserve `scriberig-store` semantics and should be deterministic, forward-compatible, and covered by recovery/integrity tests.
