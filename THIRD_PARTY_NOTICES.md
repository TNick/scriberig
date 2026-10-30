# Third-Party Notices

The repository does not currently bundle third-party source code, fonts, media,
or generated artifacts that require a repository-level notice beyond the
dependency metadata shipped by Cargo and pnpm.

Dependency-specific license and notice obligations must still be reviewed when
a dependency or distributed asset is added or upgraded. When a dependency,
asset, generated file, font, parser, or other third-party material requires a
central notice, add the exact required text here or in the package that
distributes it. Do not invent attribution requirements for material that does
not require them.

## rpi-tui

ScribeRig uses the `rpi-*` crates from `TNick/pi-rust`, pinned in Cargo
manifests to commit `b2fc63c78ae85044eb78e7ac9af184bd7fbfbc95`. This fork
revision includes official `bigfish1913/pi-rust` 0.7.1 main commit
`281dfddfa39ac063d86de57b75370974c4d75e6c`, the host-injected model catalog
refresh, transactional model configuration, cancellable provider stream, TUI
component event/focus extractions, application-registered keybindings, a host
tick callback, and upstream-owned theme preset metadata. The crates are
licensed under the MIT License; their license and notice are available from
the pinned upstream repository.
