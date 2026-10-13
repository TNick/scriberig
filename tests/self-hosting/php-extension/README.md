# PHP self extension fixture

This directory defines the source and path contract for the PHP ingestion
candidate constructed by the self-hosting provider. The normal Scriberig
workspace deliberately has no PHP domain crate or registered PHP adapter;
`.php` currently uses the generic text adapter.

Files here are test/reference data. They are not Cargo workspace members and
do not register PHP in production. The candidate source bundle will be
materialized only into an isolated candidate workspace, where its focused
tests and project import tests run before any live checkout change.

The parser dependency target is `tree-sitter-php` 0.25.1, MIT licensed, using
the same Tree-sitter 0.25 runtime as the existing Python adapter. Cargo
resolves and records the exact transitive packages in the candidate lockfile;
the fixture does not vendor parser code.

`manifest.json` lists repository-relative candidate effects and acceptance
areas. `php-project/` contains valid pure PHP and mixed inline HTML/PHP sources
that can be ingested after the candidate is applied. `fixtures/Broken.php.txt`
contains malformed parser input outside the onboarded project; the import
negative test writes malformed bytes to a temporary `.php` source file.
