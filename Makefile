.PHONY: help verify verify-all check-test check-clippy check-fmt check-build check-wasm-build build-wasm e2e-prep check-e2e check-render check-coverage gen-levels check-levels gen-props check-props gen-title

# Colors for output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
NC=\033[0m # No Color

help:
	@echo "Available targets:"
	@echo "  make verify          - Run core checks (fmt, clippy, test, build, wasm build, levels, props)"
	@echo "  make verify-all      - verify + the browser suites (check-e2e, check-render)"
	@echo "  make check-test      - Run test suite"
	@echo "  make check-clippy    - Run clippy linting"
	@echo "  make check-fmt       - Check code formatting"
	@echo "  make check-build     - Build release binary"
	@echo "  make check-wasm-build - Build for wasm32 target (compilation check only)"
	@echo "  make build-wasm      - Build WASM and generate JavaScript glue (for local testing)"
	@echo "  make e2e-prep        - Build the wasm + glue, bun install, install Chromium (shared by check-e2e / check-render)"
	@echo "  make check-e2e       - Run the Playwright end-to-end specs (tests/e2e/specs, 60 s timeout)"
	@echo "  make check-render    - Run the renderer acceptance scripts (composite-coherence + props-stability)"
	@echo "  make check-coverage  - Generate code coverage report (requires cargo-tarpaulin)"
	@echo "  make gen-levels      - Regenerate src/levels_data.rs from levels/*.json"
	@echo "  make check-levels    - Validate levels/*.json and check levels_data.rs is up to date"
	@echo "  make gen-props       - Regenerate src/props_data.rs from props/props.json"
	@echo "  make gen-title       - Regenerate the loading-screen title SVG in index.html"
	@echo "  make check-props     - Validate props/props.json and check props_data.rs is up to date"

# The core checks: fmt, clippy, tests, release build, wasm compile check,
# generated-data checks. No browser, no wasm-bindgen.
CORE_CHECKS = check-fmt check-clippy check-test check-build check-wasm-build check-levels check-props

# Run all verification checks (the browser suites are excluded by default:
# they need wasm-bindgen-cli, Bun and a Chromium — see check-e2e / check-render)
verify: $(CORE_CHECKS)
	@echo "$(GREEN)✓ All core checks passed!$(NC)"
	@echo "$(YELLOW)Note: browser suites skipped (run 'make check-e2e' / 'make check-render', or 'make verify-all')$(NC)"

# Everything: the core checks + the two browser suites
verify-all: $(CORE_CHECKS) check-e2e check-render
	@echo "$(GREEN)✓ All checks passed!$(NC)"

# Test Suite - runs all tests including doc tests
check-test:
	@echo "$(YELLOW)Running test suite...$(NC)"
	cargo test --verbose --all-features
	cargo test --doc --verbose
	@echo "$(GREEN)✓ Test suite passed$(NC)"

# Clippy - linting with warnings as errors
check-clippy:
	@echo "$(YELLOW)Running clippy (linting)...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "$(YELLOW)Running clippy for the wasm32 target (audio/graphics/input/render/camera/lib are wasm-only)...$(NC)"
	cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings
	@echo "$(GREEN)✓ Clippy passed$(NC)"

# Rustfmt - code formatting check
check-fmt:
	@echo "$(YELLOW)Checking code formatting...$(NC)"
	cargo fmt --all -- --check
	@echo "$(GREEN)✓ Formatting check passed$(NC)"

# Build - release build
check-build:
	@echo "$(YELLOW)Building release library...$(NC)"
	cargo build --release --lib --verbose
	@echo "$(GREEN)✓ Build passed$(NC)"

# WASM Build Check - verify wasm32 target compiles (catches wasm-specific issues)
check-wasm-build:
	@echo "$(YELLOW)Checking WASM compilation...$(NC)"
	@rustup target list --installed | grep -q wasm32-unknown-unknown || (echo "Installing wasm32-unknown-unknown target..." && rustup target add wasm32-unknown-unknown)
	cargo build --lib --target wasm32-unknown-unknown
	@echo "$(GREEN)✓ WASM build check passed$(NC)"

# The wasm-bindgen crate version in Cargo.lock: the CLI must match it EXACTLY
# or glue generation fails, so build-wasm installs that version when the CLI is
# missing or a different version is on PATH.
WASM_BINDGEN_VERSION = $(shell grep -A 2 'name = "wasm-bindgen"' Cargo.lock | grep '^version = ' | head -1 | sed 's/version = "\(.*\)"/\1/')

# Build WASM - build WASM and generate JavaScript glue for local testing
build-wasm:
	@echo "$(YELLOW)Building WASM for local testing...$(NC)"
	@rustup target list --installed | grep -q wasm32-unknown-unknown || (echo "Installing wasm32-unknown-unknown target..." && rustup target add wasm32-unknown-unknown)
	cargo build --release --target wasm32-unknown-unknown
	@echo "$(YELLOW)Generating wasm-bindgen JavaScript glue...$(NC)"
	@if [ "$$(wasm-bindgen --version 2>/dev/null | awk '{print $$2}')" != "$(WASM_BINDGEN_VERSION)" ]; then \
		echo "Installing wasm-bindgen-cli v$(WASM_BINDGEN_VERSION) to match Cargo.lock..."; \
		cargo install wasm-bindgen-cli --version $(WASM_BINDGEN_VERSION) --locked; \
	fi
	wasm-bindgen target/wasm32-unknown-unknown/release/open_miami.wasm --out-dir . --target web --no-typescript
	@echo "$(GREEN)✓ WASM build complete! Files generated:$(NC)"
	@echo "  - open_miami.js"
	@echo "  - open_miami_bg.wasm"
	@echo "$(YELLOW)You can now open index.html in a web browser (via a local web server)$(NC)"

# Browser suites - shared preparation: the wasm + glue (build-wasm), the Bun
# deps, the Chromium browser (+ its system libs, rootless fallback via
# setup-browser-deps.sh). Both check-e2e and check-render depend on it.
e2e-prep: build-wasm
	@echo "Installing E2E test dependencies (via Bun)..."
	cd tests/e2e && bun install
	@echo "Installing the Chromium browser (with system deps when possible)..."
	@# stdin closed: `--with-deps` shells out to sudo/su for apt and would
	@# otherwise sit on a password prompt forever on a box without root.
	cd tests/e2e && (bunx playwright install --with-deps chromium </dev/null || bunx playwright install chromium)
	@echo "Ensuring browser system libraries are present (rootless fallback)..."
	cd tests/e2e && ./setup-browser-deps.sh

# The rootless browser's extracted system libs (tests/e2e/playwright-deps/),
# prepended to LD_LIBRARY_PATH for every browser launch (run from tests/e2e).
E2E_ENV = LD_LIBRARY_PATH="$$(find "$$PWD/playwright-deps/libs" -name '*.so*' -printf '%h\n' 2>/dev/null | sort -u | paste -sd:):$${LD_LIBRARY_PATH:-}"

# E2E Tests - end-to-end tests with Playwright
# IMPORTANT: Always run via 'make check-e2e' to ensure proper timeout enforcement
# Running tests directly without timeout can cause Claude Code instances to hang
# `ulimit -c 0`: a crashing Chromium must not leave GB-sized core dumps in tests/e2e/.
check-e2e: e2e-prep
	@echo "$(YELLOW)Running end-to-end tests with 60-second timeout...$(NC)"
	cd tests/e2e && mkdir -p test-results && ulimit -c 0 && $(E2E_ENV) timeout 60 bunx playwright test
	@echo "$(GREEN)✓ E2E tests passed$(NC)"

# Render Tests - the two standalone renderer acceptance scripts
# (tests/e2e/composite-coherence.js: the smooth pixel-group composite, at DPR
# 1 and 2, ~7 s; tests/e2e/props-stability.js: the ?viz PROPS pixel-art
# stability, ~60 s — it is fixed-sleep bound: ~60 waitForTimeout calls + 9
# page loads of /?viz). Each launches its own Chromium, so they run IN
# PARALLEL against one serve.py started on RENDER_PORT for the duration of
# the target (killed on exit whatever the outcome), each under its own
# `timeout`; their output goes to tests/e2e/test-results/render-*.log and is
# printed once both are done. Too slow for `verify` — `verify-all` only.
# RENDER_PORT defaults to a FREE ephemeral port (other checkouts / worktrees
# often keep a serve.py on the scripts' own default 8098 — testing against a
# foreign tree by accident must be impossible), and the recipe fails if the
# server it started is not the one listening.
ifeq ($(origin RENDER_PORT), undefined)
RENDER_PORT := $(shell python3 -c 'import socket; s = socket.socket(); s.bind(("", 0)); print(s.getsockname()[1])')
endif
RENDER_TIMEOUT ?= 180
check-render: e2e-prep
	@echo "$(YELLOW)Running renderer acceptance tests (serve.py on :$(RENDER_PORT), $(RENDER_TIMEOUT) s timeout each)...$(NC)"
	@ulimit -c 0; \
	python3 serve.py $(RENDER_PORT) >/dev/null 2>&1 & SRV=$$!; \
	trap 'kill $$SRV 2>/dev/null' EXIT; \
	for i in $$(seq 1 50); do curl -sf -o /dev/null http://127.0.0.1:$(RENDER_PORT)/index.html && break; sleep 0.1; done; \
	kill -0 $$SRV 2>/dev/null || { echo "$(RED)serve.py did not start on :$(RENDER_PORT) (port in use?) — set RENDER_PORT$(NC)"; exit 1; }; \
	cd tests/e2e && mkdir -p test-results; \
	$(E2E_ENV) timeout $(RENDER_TIMEOUT) bun composite-coherence.js http://127.0.0.1:$(RENDER_PORT) > test-results/render-composite-coherence.log 2>&1 & P1=$$!; \
	$(E2E_ENV) timeout $(RENDER_TIMEOUT) bun props-stability.js http://127.0.0.1:$(RENDER_PORT) > test-results/render-props-stability.log 2>&1 & P2=$$!; \
	wait $$P1; R1=$$?; wait $$P2; R2=$$?; \
	echo "--- composite-coherence.js (exit $$R1)"; cat test-results/render-composite-coherence.log; \
	echo "--- props-stability.js (exit $$R2)"; cat test-results/render-props-stability.log; \
	[ $$R1 -eq 0 ] && [ $$R2 -eq 0 ]
	@echo "$(GREEN)✓ Render tests passed$(NC)"

# Levels - compile the floor/scenario JSON (levels/*.json, written by the
# level editor) into static Rust data. Python 3 stdlib only.
gen-levels:
	@echo "$(YELLOW)Generating src/levels_data.rs from levels/*.json...$(NC)"
	python3 tools/gen_levels.py
	@echo "$(GREEN)✓ Levels generated$(NC)"

# Levels check - validate the JSON and make sure the generated file is current
check-levels:
	@echo "$(YELLOW)Validating levels/*.json...$(NC)"
	python3 tools/gen_levels.py --check
	@echo "$(GREEN)✓ Levels valid and up to date$(NC)"

# Props - compile the prop library's saved pixel-art settings
# (props/props.json, written by the ?viz PROPS page SAVE) into static Rust
# data. Python 3 stdlib only.
gen-props:
	@echo "$(YELLOW)Generating src/props_data.rs from props/props.json...$(NC)"
	python3 tools/gen_props.py
	@echo "$(GREEN)✓ Props generated$(NC)"

# Props check - validate the JSON and make sure the generated file is current
check-props:
	@echo "$(YELLOW)Validating props/props.json...$(NC)"
	python3 tools/gen_props.py --check
	@echo "$(GREEN)✓ Props valid and up to date$(NC)"


# Loading-screen title - the neon OPEN/MIAMI SVG inlined into index.html,
# generated from src/lib.rs's title glyphs. Python 3 stdlib only.
gen-title:
	@echo "$(YELLOW)Generating the loading-screen title SVG...$(NC)"
	python3 tools/gen_title.py
	@echo "$(GREEN)✓ Title SVG generated$(NC)"

# Code Coverage - requires cargo-tarpaulin (optional check)
check-coverage:
	@echo "$(YELLOW)Generating code coverage...$(NC)"
	@which cargo-tarpaulin > /dev/null || (echo "$(RED)cargo-tarpaulin not installed. Run: cargo install cargo-tarpaulin$(NC)" && exit 1)
	cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out xml
	@echo "$(GREEN)✓ Coverage report generated$(NC)"
