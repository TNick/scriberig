# Web Workspace

The web workspace contains reusable public TypeScript/React packages under `packages/` and complete applications under `apps/`.

Reusable packages must not import from `web/apps/` or from Rust/server implementation internals. The intended dependency direction is protocol -> client -> model -> editor-core/react -> reusable components -> integrated application. MUI-specific dependencies belong only in `components-mui` and complete product applications.

The official editor is a conformance consumer of public packages: if it needs a private shortcut, improve the reusable public boundary instead.

TypeScript and JavaScript use spaces with an indentation width of 2. Prettier is authoritative for web source formatting; use the workspace formatter rather than hand-formatting around it.
