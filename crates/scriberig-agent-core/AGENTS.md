# Agent Core

`scriberig-agent-core` contains reusable agent-facing tool descriptions, capability exposure, semantic navigation adapters, and context construction interfaces. It is Apache-licensed reusable infrastructure.

It may consume application/session/protocol abstractions but must not execute model providers or own the integrated orchestration loop. Provider/runtime behavior belongs in `scriberig-agent-runtime`.
