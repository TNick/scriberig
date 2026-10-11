# Rust Client

`scriberig-client` is a reusable Rust client for a running Scriberig server. It depends on public protocol contracts rather than server internals.

Keep transport/client concerns here and semantic application behavior on the server/application side. Third-party Rust programs should be able to use this crate without linking the integrated agent runtime.
