# Generated DOCX Test Fixture

The DOCX package used by these tests is generated in memory by the Rust test
helpers rather than checked in as a binary. This keeps the fixture
deterministic, reviewable, and free of unexplained office-suite metadata.

`tests/import.rs::docx_fixture` creates the minimal structured document with
these anchors:

- Paragraph: `Existing site area`
- Paragraph: `Preserve this paragraph`
- Table cell: `parcel 124`

`tests/edit.rs::package` adds a paragraph style, a bookmark around the second
paragraph, an opaque custom XML member, a header, a footer, and comments. The
paragraph edit tests verify that these unrelated package members survive.
`package_with_archive_comment` adds raw ZIP archive comment bytes for the
archive-metadata preservation test.

Run the fixture import and preservation checks with:

```text
cargo test -p scriberig-domain-docx --test import
cargo test -p scriberig-domain-docx --test edit
```
