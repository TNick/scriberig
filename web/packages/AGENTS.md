# Public Web Packages

Packages in this directory are reusable library boundaries for third-party applications as well as the official editor.

Keep imports on public package exports; do not reach into another package's private `src/` paths. `protocol` has no Scriberig web dependency; `client` builds on protocol; `model` builds on client/protocol; `editor-core` is framework-independent; `react` binds reusable state to React; `components` is styling-light; `components-mui` is the opinionated MUI layer; `agent-ui` presents session/agent state without owning agent execution.

No package here may depend on `web/apps/scriberig`.
