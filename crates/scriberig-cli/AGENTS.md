# CLI

`scriberig-cli` owns command parsing and dispatch over public application/client capabilities. It is a reusable Apache-licensed surface and must not become an alternate home for business logic.

The native executable is `srig`. Its stable top-level mode names are `tui`, `serve`, `run`, `agent`, and `maintenance`, with standard help and version output. Modes may be introduced as honest stubs before their owning runtime is implemented.

Commands delegate to `scriberig-app`, `scriberig-client`, or explicitly composed product modes. Keep persistence, semantic mutation, server behavior, terminal rendering, and agent orchestration in their owning crates. Command parsing may describe and validate arguments, but it must not duplicate those implementations.
