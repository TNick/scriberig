# Protocol

`scriberig-protocol` owns stable serializable transport-facing contracts. It translates application concepts into a wire model but does not own server business logic or database implementation details.

The TypeScript `@scriberig/protocol` package must represent the same wire contract. Keep protocol specifications reusable and Apache-licensed so third-party clients do not require the integrated server/runtime source tree.
