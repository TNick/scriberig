# Product golden workflow

This directory documents the mandatory product-level acceptance gate defined
by design task 122. All listed suites use temporary fixtures and the real
browser/native process composition; no server mocks or skipped cases are
allowed.

The intended commands are:

```text
python tools/product_golden.py
make product-golden
```

The required scenario mapping is:

| ID | Required verification file | Current status |
| --- | --- | --- |
| WEB-GOLDEN-01..07 | `web/e2e/specs/golden-project-workflow.spec.ts` | passing locally |
| TUI-GOLDEN-01..04 | `apps/scriberig/tests/tui_pty.rs` | passing locally |
| TUI-GOLDEN-05 | `apps/scriberig/tests/tui_pty_golden_agent.rs` | passing locally |
| GOLDEN-CONFORMANCE-01..07 | `crates/scriberig-project/tests/product_conformance.rs` and `crates/scriberig-server/tests/remote_project_conformance.rs` | passing locally |

No required scenario is skipped or ignored by this document. Test diagnostics
include the PTY transcript and temporary fixture path on failure.
