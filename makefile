.PHONY: help check-banned lint fmt test build clean coverage-check coverage-html

# Minimum coverage threshold (%)
COVERAGE_MIN ?= 80

help:
	@echo "Available targets:"
	@echo "  check-banned    - Check for banned patterns (println!, ad-hoc logging init)"
	@echo "  lint            - Run all linting checks (banned patterns + clippy + fmt check)"
	@echo "  fmt             - Format all code"
	@echo "  doc			 - Create rustdocs
	@echo "  test            - Run all tests"
	@echo "  build           - Build all crates"
	@echo "  clean           - Clean build artifacts"
	@echo "  coverage-check  - Run tests with coverage and enforce minimum threshold"
	@echo "  coverage-html   - Generate HTML coverage report"

check-banned:
	@./scripts/check_banned_patterns.sh

lint: check-banned
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

doc:
	cargo doc --workspace --no-deps --target aarch64-apple-darwin

test:
	cargo test --workspace

build:
	cargo build --workspace

clean:
	cargo clean

coverage-check:
	@echo "Running tests with coverage (minimum threshold: $(COVERAGE_MIN)%)..."
	@cargo tarpaulin --workspace --out Xml --output-dir coverage --timeout 300 --exclude-files 'target/*' --exclude-files 'tests/*' -- --test-threads=1
	@echo "Coverage report generated: coverage/cobertura.xml"
	@echo "Checking coverage threshold..."
	@COVERAGE=$$(grep -o 'line-rate="[^"]*"' coverage/cobertura.xml | head -1 | sed 's/line-rate="\([^"]*\)"/\1/' | awk '{print int($$1 * 100)}'); \
	if [ -z "$$COVERAGE" ]; then \
		echo "❌ Could not parse coverage from cobertura.xml"; \
		exit 1; \
	elif [ "$$COVERAGE" -lt "$(COVERAGE_MIN)" ]; then \
		echo "❌ Coverage $$COVERAGE% is below minimum threshold $(COVERAGE_MIN)%"; \
		exit 1; \
	else \
		echo "✅ Coverage $$COVERAGE% meets minimum threshold $(COVERAGE_MIN)%"; \
	fi

coverage-html:
	@echo "Generating HTML coverage report..."
	@cargo tarpaulin --workspace --out Html --output-dir coverage --timeout 300 --exclude-files 'target/*' -- --test-threads=1
	@echo "✅ HTML coverage report generated: coverage/tarpaulin-report.html"
	@echo "Open with: open coverage/tarpaulin-report.html"
