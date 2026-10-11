# Agent Runtime

`scriberig-agent-runtime` is the integrated AGPL orchestration/runtime layer. It may use `scriberig-agent-core`, sessions, application operations, and provider integrations, but reusable foundation/domain crates must never depend on it.

Keep model-provider execution, orchestration loops, and product-specific runtime policy here. Reusable semantic navigation/tool descriptions belong in `scriberig-agent-core`; durable session state belongs in `scriberig-session`.
