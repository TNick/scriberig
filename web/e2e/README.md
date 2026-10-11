# Browser end-to-end tests

This suite runs the reference browser application against the repository's
managed local API and SQLite development fixture. It verifies the connected
knowledge-browser vertical slice, application navigation, explicit
unsupported states, lazy expansion, shared selection, and disconnected
bootstrap behavior.

Run it from the repository root with:

```text
pnpm --dir web e2e
```

The suite starts and stops services through `tools/dev.py`. Failed traces,
screenshots, videos, and the HTML report are written below
`output/playwright/`.
