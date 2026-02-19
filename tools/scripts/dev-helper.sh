#!/bin/bash
# File: ~/Development/tools/scripts/dev-helper.sh (Updated)

set -e

COMMAND=$1
shift

case $COMMAND in
    "lint-rust")
        echo "🔍 Running Rust linting..."
        cargo clippy --workspace --all-targets --all-features -- -D warnings
        cargo fmt --all -- --check
        taplo fmt --check Cargo.toml */Cargo.toml
        ;;
    "fix-rust")
        echo "🔧 Auto-fixing Rust issues..."
        cargo clippy --workspace --fix --allow-dirty --allow-staged
        cargo fmt --all
        taplo fmt Cargo.toml */Cargo.toml
        cargo sort --workspace --grouped
        ;;
    "lint-frontend")
        echo "🔍 Running frontend linting..."
        if [ -f "package.json" ]; then
            pnpm lint
            pnpm prettier --check .
            pnpm stylelint "**/*.{css,scss,svelte}"
            pnpm svelte-check --tsconfig ./tsconfig.json
        else
            echo "❌ No package.json found"
            exit 1
        fi
        ;;
    "fix-frontend")
        echo "🔧 Auto-fixing frontend issues..."
        if [ -f "package.json" ]; then
            pnpm lint --fix
            pnpm prettier --write .
            pnpm stylelint "**/*.{css,scss,svelte}" --fix
        else
            echo "❌ No package.json found"
            exit 1
        fi
        ;;
    "lint-all")
        echo "🔍 Running all linting..."
        $0 lint-rust
        if [ -f "package.json" ]; then
            $0 lint-frontend
        fi
        ;;
    "fix-all")
        echo "🔧 Auto-fixing all issues..."
        $0 fix-rust
        if [ -f "package.json" ]; then
            $0 fix-frontend
        fi
        ;;
    "security-audit")
        echo "🔒 Running security audits..."
        cargo audit
        if [ -f "package.json" ]; then
            pnpm audit
        fi
        ;;
    "deps-check")
        echo "📦 Checking dependencies..."
        cargo outdated
        cargo machete
        if [ -f "package.json" ]; then
            pnpm outdated
        fi
        ;;
    # ... (previous commands remain the same)
    *)
        echo "Usage: $0 {lint-rust|fix-rust|lint-frontend|fix-frontend|lint-all|fix-all|security-audit|deps-check|...}"
        echo ""
        echo "Linting & Formatting:"
        echo "  lint-rust      - Run Rust clippy and fmt check"
        echo "  fix-rust       - Auto-fix Rust formatting and simple issues"
        echo "  lint-frontend  - Run ESLint, Prettier, and Stylelint checks"
        echo "  fix-frontend   - Auto-fix frontend formatting and simple issues"
        echo "  lint-all       - Run all linting checks"
        echo "  fix-all        - Auto-fix all formatting and simple issues"
        echo "  security-audit - Run security vulnerability scans"
        echo "  deps-check     - Check for outdated and unused dependencies"
        echo ""
        # ... (previous help text)
        exit 1
        ;;
esac