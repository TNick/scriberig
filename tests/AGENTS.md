# Tests

This directory owns tests that intentionally span multiple packages or product surfaces.

- `conformance/` verifies that independent implementations obey one shared contract.
- `protocol/` holds wire-contract fixtures and cross-language compatibility tests.
- `end-to-end/` verifies complete vertical capabilities across supported surfaces.
- `fixtures/` contains stable, minimal input data shared by those tests.

Unit tests that belong to one crate or web package should live beside that package instead of here. Cross-layer tests must use public APIs and must not reach into private storage tables or private web package source paths.
