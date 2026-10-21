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
manifests to commit `b78a199486351d86804995c27a0c5ed2bd8a28e3`. This fork
revision includes official `bigfish1913/pi-rust` main commit
`7a272ecf42e5ec17efedc92031a5cdba85e213e9` and the host-injected model
catalog refresh, transactional model configuration, cancellable provider
stream, TUI component event/focus extractions, and application-registered
keybindings with shared help/footer enumeration. The crates are
licensed under the MIT License; their license and notice are available from
the pinned upstream repository.
