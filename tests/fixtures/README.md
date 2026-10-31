# Shared fixtures

`document-paragraph-edit.json` is a stable transport-level request and response
contract consumed by the web client and TUI remote-adapter tests. It describes
an edit to the first paragraph (`Existing site area`) in
`rich-project/report.docx`. The candidate identity is representative test data;
the TUI application-adapter integration and browser end-to-end test use the same
paragraph and replacement against real onboarded copies. The browser test must
pass before cross-surface parity is considered verified.

`agent-selection-normalization.json` defines the semantic selection and
provider-neutral wire payload for spreadsheet-cell and CRS-aware geospatial
feature cases. Web selection normalization, web client serialization, and the
TUI embedded-to-remote adapter all consume this contract to prove view anchors
do not alter the model scope.
