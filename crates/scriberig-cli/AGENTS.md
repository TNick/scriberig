# CLI

`scriberig-cli` owns command parsing and dispatch over public application/client capabilities. It is a reusable Apache-licensed surface and must not become an alternate home for business logic.

Commands delegate to `scriberig-app`, `scriberig-client`, or explicitly composed product modes. Keep persistence, semantic mutation, and agent orchestration in their owning crates.
