.PHONY: help check-banned lint fmt test build clean

help:
	@echo "Available targets:"
	@echo "  check-banned  - Check for banned patterns (println!, ad-hoc logging init)"
	@echo "  lint          - Run all linting checks (banned patterns + clippy + fmt check)"
	@echo "  fmt           - Format all code"
	@echo "  test          - Run all tests"
	@echo "  build         - Build all crates"
	@echo "  clean         - Clean build artifacts"

check-banned:
	@./scripts/check_banned_patterns.sh

lint: check-banned
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo test --workspace

build:
	cargo build --workspace

clean:
	cargo clean
