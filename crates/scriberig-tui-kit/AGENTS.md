# TUI Kit

This crate is a transitional compatibility layer. Corrective plan 032 makes
upstream `rpi-tui` the terminal framework owner; task 144 records this facade
and its commodity wrappers for deletion in tasks 147–149. Do not add widgets,
runtime facilities, or new facade surface here. Scriberig-specific views should
become thin `rpi-tui::Component` adapters; semantic foundation crates must not
depend on `rpi-tui`.

Keep Scriberig semantic types out of this crate. Convert external text to safe
plain data before passing it to upstream rendering components during the
transition.
