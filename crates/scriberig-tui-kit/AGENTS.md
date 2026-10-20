# TUI Kit

This crate adapts reusable terminal presentation primitives for Scriberig.

Keep Scriberig semantic types out of this crate. Contain upstream `rpi-tui`
types behind the public facade, and convert external text to safe plain data
before passing it to upstream rendering components.
