# Schema Layer

`scriberig-schema` defines domain-independent entity/property/relation/collection schema vocabulary and structured validation contracts. Concrete production domain schemas live in `crates/domains/`.

This crate may depend on universal core types but must not depend on graph persistence, agents, transports, UI, or one concrete domain.
