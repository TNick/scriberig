# Editor Core

`@scriberig/editor-core` is framework-independent interaction logic for semantic browsing/editing: universal selection, tree expansion, navigation, command discovery, diffs/history, branch navigation, migration review, operation progress, and projection coordination.

Do not add JSX, React state, MUI types, transport details, or product-shell assumptions here. React bindings consume these controllers; visual packages render them; the integrated app composes them.
