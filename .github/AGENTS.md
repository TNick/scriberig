# GitHub Automation

This directory contains repository-hosted automation such as CI workflows.

CI must exercise the same quality gates developers run locally: repository conventions, Rust formatting/checks/clippy, and web formatting/type checks. Workflows may orchestrate tooling but must not become the only place where a rule exists; keep reusable checks under `tools/` when practical.
