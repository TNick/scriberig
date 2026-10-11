# Derived Knowledge

`scriberig-derive` owns reusable mechanics for derivation profiles, exact dependency keys, immutable derived results, freshness/retention, invalidation, explanation, events, and derivation registration.

Derived state must not silently become canonical state or advance semantic snapshots. Domain adapters register derivations here; model providers, compiler integrations, server transport, and UI remain outside this crate.
