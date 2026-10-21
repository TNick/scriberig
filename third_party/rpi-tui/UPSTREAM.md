# Upstream Provenance

## Source

- Repository: <https://github.com/bigfish1913/pi-rust>
- Commit: `aab74de0737f4b7c454cd8c8c16ca66e4dfbf898`
- Commit date: 2026-10-19 23:47:39 UTC
- Upstream crate: `crates/rpi-tui`
- Upstream crate tree at the pinned commit: `6574114fa2040dcf4738f601b6ecf948bf2ab5ca`
- Upstream license: MIT, as recorded by the root `LICENSE` file.
- Upstream notice: root `NOTICE` file.
- Intake date: 2026-10-20.

The source was acquired from the public upstream Git repository using a sparse
checkout at the exact commit above. The complete crate `src/`, `tests/`,
`Cargo.toml`, and `README.md` are preserved under `crate/`. The upstream root
`LICENSE` and `NOTICE` are preserved beside this file.

The upstream root `SECURITY.md` was inspected at the pinned commit for its
guidance on terminal input, remote operation, tools, plugins, and credentials.
It is not redistributed because it is upstream project policy rather than
crate source or a notice required for this intake.

## Omitted Files

The upstream crate's examples were omitted because they are demo composition
code and are not needed to build or test the library. Other `pi-rust` workspace
crates and workspace files were omitted; `rpi-tui` has no dependency on them.

## Reproducing and Updating

To verify the original source, check out the recorded commit from the upstream
repository and compare its `crates/rpi-tui` tree with `crate/`, accounting for
the manifest changes listed in `PATCHES.md`.

## Vendor Update Procedure

1. Select a new exact upstream commit; do not pin a moving branch or tag.
2. Inspect that revision's `LICENSE`, `NOTICE`, security guidance, crate
   manifest, and relevant release history.
3. Compare the previous and proposed `crates/rpi-tui` source trees, tests, and
   dependency graph.
4. Reapply the packaging changes listed in `PATCHES.md` deliberately. Review
   every source change separately and preserve upstream attribution.
5. Update this file with the new SHA, commit date, crate tree hash, intake date,
   license, acquisition method, and any changed omissions.
6. Update `PATCHES.md` so it describes every remaining manifest or source
   difference from that exact revision.
7. Run focused `scriberig-tui-kit` tests and checks.
8. Run focused `scriberig-tui` tests and checks, including terminal safety and
   lifecycle coverage.
9. Inspect the final dependency graph and verify one compatible Crossterm line
   is used by the active TUI path. Keep image support disabled and do not add
   upstream agent, harness, tool, or plugin crates.
10. Review and commit the upstream update separately where practical so the
    provenance change is easy to inspect and revert.

Ordinary builds and CI consume only the vendored crate and do not fetch
upstream source. Do not add other `rpi-*` crates to Scriberig as part of an
upstream update.
