#!/bin/bash
# File: ~/Development/tools/scripts/setup-git-hooks.sh

set -e

PROJECT_ROOT=${1:-$(pwd)}

if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "❌ Not in a git repository"
    exit 1
fi

# Create pre-commit hook
cat > "$PROJECT_ROOT/.git/hooks/pre-commit" << 'EOF'
#!/bin/bash

set -e

echo "🔍 Running pre-commit checks..."

# Check Rust code
if find . -name "*.rs" -not -path "./target/*" | grep -q .; then
    echo "Checking Rust code..."
    cargo fmt -- --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
fi

# Check frontend code
if [ -f "package.json" ]; then
    echo "Checking frontend code..."
    pnpm lint
    pnpm prettier --check .
    pnpm svelte-check --tsconfig ./tsconfig.json
fi

# Check TOML files
if command -v taplo &> /dev/null; then
    echo "Checking TOML files..."
    taplo fmt --check Cargo.toml */Cargo.toml 2>/dev/null || true
fi

echo "✅ Pre-commit checks passed!"
EOF

chmod +x "$PROJECT_ROOT/.git/hooks/pre-commit"

echo "✅ Git hooks installed successfully!"