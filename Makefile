.PHONY: deps deps-visual-qa build release run bundle-mac test test-core test-ui test-lsp test-mcp test-parser check clippy fmt fmt-check audit verify hooks clean help

help:
	@echo "make verify          - run before pushing: fmt-check, clippy, test, audit"
	@echo "make hooks           - point git at .githooks (pre-commit + pre-push)"
	@echo ""
	@echo "make deps           - install system packages needed to build/run Protide (Ubuntu/Debian)"
	@echo "make deps-visual-qa - install extra tooling for Wayland screenshot/input automation (grim, ydotool)"
	@echo "make build           - debug build"
	@echo "make release         - release build (recommended for actually running the app)"
	@echo "make run             - cargo run --release"
	@echo "make bundle-mac      - build target/Protide.app (macOS: needed for the Dock/Finder icon)"
	@echo "make test            - run the full workspace test suite, crate by crate"
	@echo "make check           - cargo check across the whole workspace, crate by crate"
	@echo "make clippy          - cargo clippy crate by crate, warnings are errors"
	@echo "make fmt             - cargo fmt --all"
	@echo "make fmt-check       - cargo fmt --all --check (no writes)"
	@echo "make audit           - cargo audit (skipped with a notice if cargo-audit is missing)"
	@echo "make clean           - cargo clean"
	@echo ""
	@echo "Crate-by-crate (not --workspace): building the whole workspace in one cargo"
	@echo "invocation unifies feature flags across all members and can drag in an"
	@echo "optional native-TLS backend that needs extra system packages beyond what's"
	@echo "installed by 'make deps'. Building/testing one crate at a time avoids that."

# GPUI's actual Linux dependency list, taken from Zed upstream (the engine
# Protide is built on - see CLAUDE.md's "GPUI Reference" section) at
# script/linux, trimmed of Zed-specific bits (musl cross-compile toolchain,
# webrtc/remote-dev extras) that Protide doesn't use.
deps:
	sudo apt-get update
	sudo apt-get install -y \
		gcc g++ clang lld llvm make cmake build-essential \
		libssl-dev \
		libfontconfig-dev \
		libgit2-dev \
		libglib2.0-dev \
		libva-dev \
		libvulkan1 \
		libwayland-dev \
		libx11-xcb-dev \
		libxkbcommon-x11-dev \
		libzstd-dev \
		libsqlite3-dev \
		libasound2-dev \
		pipewire \
		xdg-desktop-portal \
		jq git curl

# Optional: only needed for driving/screenshotting the live app under Wayland
# (e.g. for a visual-glitch audit) - not required to build or run Protide.
deps-visual-qa:
	sudo apt-get update
	sudo apt-get install -y grim ydotool
	@echo "ydotool needs its daemon running once per session: 'sudo ydotoold &' (or as a systemd service)"

build:
	cargo build -p http-parser -p protide-core -p protide-ui -p protide-lsp -p protide-mcp -p protide

release:
	cargo build --release -p protide

run:
	cargo run --release -p protide

bundle-mac: release
	packaging/macos/bundle.sh

test: test-core test-ui test-lsp test-mcp test-parser

test-core:
	cargo test -p protide-core --lib --features full-sync

test-ui:
	cargo test -p protide-ui --lib

test-lsp:
	cargo test -p protide-lsp --bin protide-lsp

test-mcp:
	cargo test -p protide-mcp

test-parser:
	cargo test -p http-parser --lib

check:
	cargo check -p http-parser -p protide-core -p protide-ui -p protide-lsp -p protide-mcp -p protide

clippy:
	cargo clippy -p http-parser --all-targets -- -D warnings
	cargo clippy -p protide-core --all-targets -- -D warnings
	cargo clippy -p protide-ui --all-targets -- -D warnings
	cargo clippy -p protide-lsp --all-targets -- -D warnings
	cargo clippy -p protide-mcp --all-targets -- -D warnings
	cargo clippy -p protide --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

# Advisory DB scan. cargo-audit is optional tooling, so a missing binary is a
# notice rather than a failed verify. Ignore list lives in .cargo/audit.toml.
audit:
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo "SKIP: cargo-audit not installed - run 'cargo install cargo-audit' to enable"; \
	fi

# The one command to run before pushing. Ordered cheapest-first so it fails fast.
verify: fmt-check clippy test audit

hooks:
	git config core.hooksPath .githooks
	@echo "git hooks enabled: .githooks/pre-commit (fmt + clippy), .githooks/pre-push (make verify)"

clean:
	cargo clean
