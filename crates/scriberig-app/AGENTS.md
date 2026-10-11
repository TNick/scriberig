# Application Layer

`scriberig-app` is the shared application-operation boundary for CLI, server, TUI, web transport, and agents. It coordinates reusable graph/query/change/projection/session capabilities without becoming a product UI or transport layer.

User-level capabilities should be implemented here or below so all surfaces can expose the same behavior. No caller should bypass this layer to mutate storage directly. `scriberig-app` may depend on selected reusable domain adapters; foundation crates must never depend on it.
