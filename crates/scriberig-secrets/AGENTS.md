# Secrets

This Apache-2.0 reusable boundary owns protected buffers, reference resolution,
OS-store adapters, and destination policy. It must not depend on runtime,
semantic storage, CLI, or protocol crates. Never serialize secret values or
include backend errors in public diagnostics. Tests use fake stores and keys;
real OS-store tests must be explicitly selected and remove isolated entries.
