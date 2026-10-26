# Local Vendor Patches

The upstream Rust source and its tests are preserved without local source
changes. The following manifest adaptations are required to use this crate in
Scriberig:

- Replaced inherited workspace package metadata with the values from the
  selected upstream workspace at the pinned commit.
- Replaced inherited `thiserror`, `tracing`, and Tokio dependency versions with
  explicit compatible version requirements.
- Changed Crossterm from `0.27` to `0.28` so this vendored crate and the active
  `scriberig-tui` resolve one compatible Crossterm line.
- Excluded the standalone vendor package from the Scriberig workspace so it
  does not inherit Scriberig lint or package settings.
- Left upstream's optional `image-support` feature disabled by default; the
  Scriberig facade does not expose image rendering.

These are packaging/dependency adaptations only. Any future source change must
be listed here with its reason and focused regression coverage.
