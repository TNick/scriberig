# Agent Runtime

`scriberig-agent-runtime` is the integrated AGPL orchestration/runtime layer. It may use `scriberig-agent-core`, sessions, application operations, and provider integrations, but reusable foundation/domain crates must never depend on it.

Keep Scriberig-specific branch orchestration, context compilation, semantic capability policy, and adapters here. Generic provider implementations and the ordinary in-branch model/tool loop are upstream `rpi-ai`/`rpi-agent` concerns under `design/level-2/032. upstream first agent runtime and terminal product ownership correction.md`; task 144 records the deletion and extraction gates. Do not add replacement local owners for generic provider, model, or loop machinery. Reusable semantic navigation/tool descriptions belong in `scriberig-agent-core`; durable session state belongs in `scriberig-session`.
