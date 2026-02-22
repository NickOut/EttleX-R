#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "Checking for banned patterns..."

# Check for println!/eprintln! outside tests and examples (excluding doc comments)
echo "Checking for println!/eprintln! in non-test code..."
PRINTLN_VIOLATIONS=$(git grep -n 'println!\|eprintln!' -- 'crates/*/src/**/*.rs' ':!crates/*/src/**/*test*.rs' ':!crates/*/examples/**' ':!crates/*/tests/**' | grep -v '^\([^:]*\):[0-9]*:\s*//' || true)

if [ -n "$PRINTLN_VIOLATIONS" ]; then
    echo -e "${RED}ERROR: Found println!/eprintln! in non-test code:${NC}"
    echo "$PRINTLN_VIOLATIONS"
    exit 1
fi

# Check for ad-hoc tracing initialization
echo "Checking for ad-hoc logging initialization..."
INIT_VIOLATIONS=$(git grep -n 'tracing_subscriber::.*\.init()' -- 'crates/*/src/**/*.rs' ':!crates/*/src/logging_facility/**' || true)

if [ -n "$INIT_VIOLATIONS" ]; then
    echo -e "${RED}ERROR: Found ad-hoc tracing initialization outside logging facility:${NC}"
    echo "$INIT_VIOLATIONS"
    exit 1
fi

echo -e "${GREEN}✓ Banned pattern checks passed${NC}"
