# Scriberig Server

This crate adapts remote protocol requests and event subscriptions onto shared `scriberig-app` capabilities. It is an integrated AGPL product/runtime component.

Do not place core query, migration, projection, validation, or session semantics here. Server handlers translate transport concerns into public application operations and protocol responses/events.
