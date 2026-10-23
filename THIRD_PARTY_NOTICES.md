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

## rpi-* upstream runtime crates

ScribeRig uses the `rpi-ai`, `rpi-agent`, `rpi-extensions`, `rpi-model-runtime`,
and `rpi-tui` crates from `TNick/pi-rust`, pinned in `Cargo.toml` and
`Cargo.lock` to commit `587c63f3a521567b2ef8fe9ce038fd35e3896f15`. That
controlled-fork revision merges official `bigfish1913/pi-rust` 0.7.1 main commit
`281dfddfa39ac063d86de57b75370974c4d75e6c` and adds the upstream-ownership
contributions ScribeRig consumes: the host-injected model catalog refresh,
transactional model configuration, cancellable provider stream lifecycle,
focused-component event routing and application-registered keybindings, a host
tick callback, upstream-owned theme preset metadata, validated tree item
replacement, ConPTY key-release pairing, and the isolated plugin worker/sandbox
launcher boundary. The crates are licensed under the MIT License; that pinned
revision carries the `LICENSE` and `NOTICE` texts, including the notice that the
`rpi-*` family is a Rust re-implementation of the MIT-licensed TypeScript `pi`
SDK, so no repository-level copy is required. ScribeRig vendors no upstream
source tree.
