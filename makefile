# File: Makefile (Project Root)
.PHONY: help install dev build test lint fix clean audit deps coverage coverage-html coverage-check

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*## "}; /^[a-zA-Z_-]+:.*## / {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

install: ## Install all dependencies
	@echo "📦 Installing Rust dependencies..."
	@cargo fetch
	@if [ -f "package.json" ]; then \
		echo "📦 Installing frontend dependencies..."; \
		pnpm install; \
	fi

dev: ## Start development environment
	@echo "🚀 Starting development services..."
	@./tools/scripts/dev-helper.sh start-services
	@if [ -f "src-tauri/Cargo.toml" ]; then \
		pnpm tauri dev; \
	else \
		cargo watch -x run; \
	fi

build: ## Build for production
	@echo "🏗️ Building for production..."
	@if [ -f "src-tauri/Cargo.toml" ]; then \
		pnpm tauri build; \
	else \
		cargo build --release; \
	fi

test: ## Run all tests
	@echo "🧪 Running tests..."
	@cargo nextest run --workspace
	@if [ -f "package.json" ]; then \
		pnpm test; \
	fi

lint: ## Run all linting checks
	@echo "🔍 Running linting checks..."
	@cargo clippy --workspace --all-targets --all-features -- -D warnings
	@cargo fmt --all -- --check
	@taplo fmt --check Cargo.toml */Cargo.toml
	@if [ -f "package.json" ]; then \
		pnpm lint; \
		pnpm prettier --check .; \
		pnpm svelte-check --tsconfig ./tsconfig.json; \
	fi

fix: ## Auto-fix all formatting and simple issues
	@echo "🔧 Auto-fixing issues..."
	@cargo clippy --workspace --fix --allow-dirty --allow-staged
	@cargo fmt --all
	@taplo fmt Cargo.toml */Cargo.toml
	@cargo sort --workspace --grouped
	@if [ -f "package.json" ]; then \
		pnpm lint --fix; \
		pnpm prettier --write .; \
	fi

# ... existing code ...

coverage: ## Generate Rust code coverage report (requires cargo-llvm-cov)
	@echo "📈 Generating Rust coverage (lcov)..."
	@command -v cargo-llvm-cov >/dev/null 2>&1 || (echo "cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov" && exit 1)
	@mkdir -p coverage
	@set -e; \
	LLVM_COV=""; \
	LLVM_PROFDATA=""; \
	if command -v rustup >/dev/null 2>&1; then \
		LLVM_COV="$$(rustup which llvm-cov 2>/dev/null || true)"; \
		LLVM_PROFDATA="$$(rustup which llvm-profdata 2>/dev/null || true)"; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			BREW_LLVM_PREFIX="$$(brew --prefix llvm 2>/dev/null || true)"; \
			if [ -n "$$BREW_LLVM_PREFIX" ]; then \
				if [ -x "$$BREW_LLVM_PREFIX/bin/llvm-cov" ] && [ -x "$$BREW_LLVM_PREFIX/bin/llvm-profdata" ]; then \
					LLVM_COV="$$BREW_LLVM_PREFIX/bin/llvm-cov"; \
					LLVM_PROFDATA="$$BREW_LLVM_PREFIX/bin/llvm-profdata"; \
				fi; \
			fi; \
		fi; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		echo "error: could not find llvm-cov/llvm-profdata."; \
		echo "Fix: install rustup llvm tools (rustup component add llvm-tools-preview)"; \
		echo "  or install Homebrew llvm (brew install llvm)."; \
		exit 1; \
	fi; \
	LLVM_COV="$$LLVM_COV" LLVM_PROFDATA="$$LLVM_PROFDATA" \
	cargo llvm-cov --workspace --all-features --lcov --no-cfg-coverage --output-path coverage/lcov.info

COVERAGE_MIN ?= 80

coverage-check: ## Fail if Rust line coverage is below COVERAGE_MIN (default: 80)
	@echo "✅ Checking Rust coverage (min lines: $(COVERAGE_MIN)%)..."
	@command -v cargo-llvm-cov >/dev/null 2>&1 || (echo "cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov" && exit 1)
	@set -e; \
	LLVM_COV=""; \
	LLVM_PROFDATA=""; \
	if command -v rustup >/dev/null 2>&1; then \
		LLVM_COV="$$(rustup which llvm-cov 2>/dev/null || true)"; \
		LLVM_PROFDATA="$$(rustup which llvm-profdata 2>/dev/null || true)"; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			BREW_LLVM_PREFIX="$$(brew --prefix llvm 2>/dev/null || true)"; \
			if [ -n "$$BREW_LLVM_PREFIX" ]; then \
				if [ -x "$$BREW_LLVM_PREFIX/bin/llvm-cov" ] && [ -x "$$BREW_LLVM_PREFIX/bin/llvm-profdata" ]; then \
					LLVM_COV="$$BREW_LLVM_PREFIX/bin/llvm-cov"; \
					LLVM_PROFDATA="$$BREW_LLVM_PREFIX/bin/llvm-profdata"; \
				fi; \
			fi; \
		fi; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		echo "error: could not find llvm-cov/llvm-profdata."; \
		echo "Fix: rustup component add llvm-tools-preview  (or: brew install llvm)"; \
		exit 1; \
	fi; \
	LLVM_COV="$$LLVM_COV" LLVM_PROFDATA="$$LLVM_PROFDATA" \
	cargo llvm-cov --workspace --all-features --fail-under-lines $(COVERAGE_MIN)

coverage-html: ## Generate Rust HTML coverage report (requires cargo-llvm-cov)
	@echo "📊 Generating Rust HTML coverage report..."
	@command -v cargo-llvm-cov >/dev/null 2>&1 || (echo "cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov" && exit 1)
	@mkdir -p coverage/html
	@set -e; \
	LLVM_COV=""; \
	LLVM_PROFDATA=""; \
	if command -v rustup >/dev/null 2>&1; then \
		LLVM_COV="$$(rustup which llvm-cov 2>/dev/null || true)"; \
		LLVM_PROFDATA="$$(rustup which llvm-profdata 2>/dev/null || true)"; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			BREW_LLVM_PREFIX="$$(brew --prefix llvm 2>/dev/null || true)"; \
			if [ -n "$$BREW_LLVM_PREFIX" ]; then \
				if [ -x "$$BREW_LLVM_PREFIX/bin/llvm-cov" ] && [ -x "$$BREW_LLVM_PREFIX/bin/llvm-profdata" ]; then \
					LLVM_COV="$$BREW_LLVM_PREFIX/bin/llvm-cov"; \
					LLVM_PROFDATA="$$BREW_LLVM_PREFIX/bin/llvm-profdata"; \
				fi; \
			fi; \
		fi; \
	fi; \
	if [ -z "$$LLVM_COV" ] || [ -z "$$LLVM_PROFDATA" ]; then \
		echo "error: could not find llvm-cov/llvm-profdata."; \
		echo "Fix: rustup component add llvm-tools-preview  (or: brew install llvm)"; \
		exit 1; \
	fi; \
	LLVM_COV="$$LLVM_COV" LLVM_PROFDATA="$$LLVM_PROFDATA" \
	cargo llvm-cov --workspace --all-features --html --output-dir coverage/html

clean: ## Clean build artifacts
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@sccache --zero-stats
	@if [ -f "package.json" ]; then \
		rm -rf node_modules dist .svelte-kit build; \
	fi

audit: ## Run security audits
	@echo "🔒 Running security audits..."
	@cargo audit
	@if [ -f "package.json" ]; then \
		pnpm audit; \
	fi

deps: ## Check and update dependencies
	@echo "📦 Checking dependencies..."
	@cargo outdated
	@cargo machete
	@if [ -f "package.json" ]; then \
		pnpm outdated; \
	fi

setup-hooks: ## Install Git hooks
	@echo "🪝 Setting up Git hooks..."
	@./tools/scripts/setup-git-hooks.sh
