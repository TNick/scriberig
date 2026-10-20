.DEFAULT_GOAL := help
.PHONY: help init build install tui doctor onboard clean-temp test-tui product-golden docs format lint test check dev dev-stop

ONBOARD_DB ?= playground/dev-app-runtime/scriberig.sqlite

ifeq ($(OS),Windows_NT)
PYTHON ?= python
else
PYTHON ?= python3
endif

# List the available developer commands.
help:
	@echo Scriberig developer commands:
	@echo   make init       Install web and pre-commit dependencies.
	@echo   make build      Build all workspace targets.
	@echo   make install    Build and install the srig executable.
	@echo   make tui        Onboard and open the current project in the TUI.
	@echo   make doctor     Check the configured model API and credential.
	@echo   make onboard    Import Git-tracked files from the current project.
	@echo   make clean-temp Remove stale Scriberig test workspaces from system temp.
	@echo   make test-tui   Run deterministic and PTY TUI test suites.
	@echo   make product-golden Run browser, PTY, and conformance golden tests.
	@echo   make docs       Build Rust and TypeScript API documentation.
	@echo   make format     Apply repository, Rust, and web formatting.
	@echo   make lint       Run repository and web formatting checks.
	@echo   make test       Run Rust and web test suites.
	@echo   make check      Run the full quality gate.
	@echo   make dev        Idempotently start API and web app, then open browser.
	@echo   make dev-stop   Stop processes started by make dev.

# Install the workspace's web and pre-commit dependencies.
init:
	corepack enable
	pnpm --dir web install
	$(PYTHON) -m pip install pre-commit
	pre-commit install
	pre-commit install-hooks

# Build every Rust workspace target.
build:
	cargo build --workspace --all-targets

# Build and install the srig executable into Cargo's bin directory.
install:
	cargo install --path apps/scriberig --locked --force

# Start the integrated srig executable in TUI mode.
tui:
	cargo run --package scriberig --bin srig -- tui --debug --db "$(ONBOARD_DB)" --project . --git-tracked --import

# Check the configured provider using the bounded doctor request.
doctor:
	cargo run --package scriberig --bin srig -- doctor

# Scan and import the current project's Git-tracked files.
onboard:
	$(PYTHON) tools/onboard.py --root . --db "$(ONBOARD_DB)"

# Remove Scriberig temp workspaces unchanged for at least one hour.
clean-temp:
	$(PYTHON) tools/cleanup_temp.py

# Run deterministic renderer/state tests and native PTY workflow tests.
test-tui:
	cargo test -p scriberig-tui --lib
	cargo test -p scriberig --test tui_pty

# Run the mandatory product-level golden workflow gate.
product-golden:
	$(PYTHON) tools/product_golden.py

# Build API documentation from Rust and TypeScript declaration docs.
docs:
	cargo doc --workspace --no-deps
	pnpm --dir web run docs

# Apply deterministic source and language-specific formatting.
format:
	$(PYTHON) tools/lint_repo.py --fix
	cargo fmt --all
	pnpm --dir web format

# Check repository conventions and web formatting.
lint:
	$(PYTHON) tools/lint_repo.py
	pnpm --dir web format:check

# Run the Rust and web test suites.
test: clean-temp
	cargo test --workspace
	pnpm --dir web test

# Run the complete non-mutating quality gate.
check: clean-temp
	$(PYTHON) tools/lint_repo.py
	cargo fmt --all -- --check
	cargo check --workspace --all-targets
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	pnpm --dir web format:check
	pnpm --dir web typecheck
	pnpm --dir web test

# Start or reuse the API and web app, then open the browser.
dev:
	$(PYTHON) tools/dev.py start

# Stop the services managed by make dev.
dev-stop:
	$(PYTHON) tools/dev.py stop
