.PHONY: help check-banned lint fmt test test-slice test-full build clean coverage-check coverage-html

# Minimum coverage threshold (%)
COVERAGE_MIN ?= 80

help:
	@echo "Available targets:"
	@echo "  check-banned    - Check for banned patterns (println!, ad-hoc logging init)"
	@echo "  lint            - Run all linting checks (banned patterns + clippy + fmt check)"
	@echo "  fmt             - Format all code"
	@echo "  doc             - Create rustdocs"
	@echo "  test            - Run all tests (full suite)"
	@echo "  test-slice      - Run only slice-registered tests"
	@echo "  test-full       - Run full test suite (alias for test)"
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

# Slice test targets — driven by handoff/slice_registry.toml
SLICE_TEST_FILTER ?= -E 'test(/test_ex_error_builder_all_fields|test_assert_err_kind_passes_on_correct_kind|test_assert_err_kind_fails_on_wrong_kind|test_already_tombstoned_code|test_self_referential_link_code|test_has_active_dependants_code|test_missing_link_type_code|test_new_variants_distinct|test_init_development_no_panic|test_init_test_capture_returns_handle|test_log_op_start_emits_start_event|test_log_op_end_emits_end_event_with_duration|test_log_op_error_emits_error_event_with_err_kind|test_check_banned_patterns_rejects_println|test_check_banned_patterns_rejects_tracing_subscriber_init|test_ettlex_errors_no_core_dep|test_ettlex_logging_no_core_dep|test_ettlex_x_error_not_in_workspace|test_store_public_api_no_ettlex_x_error|test_engine_public_api_no_ettlex_x_error|test_no_direct_tracing_subscriber_init|test_no_println_in_non_test_code|test_from_bridge_not_in_workspace|test_ettlex_x_error_enum_not_defined|test_core_types_correlation_types_present|test_core_types_sensitive_t_present|test_core_types_no_workspace_deps|test_migration_012_applies_cleanly|test_existing_ettle_rows_survive_with_defaults|test_create_minimal_ettle_succeeds|test_create_returns_ettle_id|test_create_with_all_fields_succeeds|test_create_with_reasoning_link_succeeds|test_create_empty_title_fails|test_create_rejects_caller_supplied_id|test_create_link_without_type_fails|test_create_type_without_link_fails|test_create_link_to_nonexistent_ettle_fails|test_create_link_to_tombstoned_ettle_fails|test_create_whitespace_only_title_fails|test_get_returns_all_fields|test_get_nonexistent_returns_not_found|test_list_empty_returns_empty_page|test_list_single_ettle|test_list_pagination_cursor|test_list_limit_zero_fails|test_list_limit_over_500_fails|test_list_invalid_cursor_fails|test_list_excludes_tombstoned_by_default|test_list_include_tombstoned_flag|test_update_title_succeeds|test_update_why_succeeds|test_update_what_succeeds|test_update_how_succeeds|test_update_sets_reasoning_link|test_update_changes_reasoning_link|test_update_clears_reasoning_link|test_update_preserves_unspecified_fields|test_update_rejects_self_referential_link|test_update_nonexistent_ettle_fails|test_update_tombstoned_ettle_fails|test_update_empty_update_fails|test_update_link_to_nonexistent_fails|test_update_link_without_type_fails|test_tombstone_active_ettle_succeeds|test_tombstone_nonexistent_ettle_fails|test_tombstone_already_tombstoned_fails|test_tombstone_with_active_dependants_fails|test_tombstone_allows_tombstoned_dependant|test_hard_delete_not_exposed|test_occ_correct_version_succeeds|test_occ_wrong_version_fails|test_each_mutation_appends_one_provenance_event|test_failed_command_no_provenance_event|test_ettle_get_byte_identical|test_ettle_list_byte_identical|test_create_large_fields_succeeds|test_list_max_limit_succeeds|test_dispatch_no_ettle_business_logic|test_dedicated_handler_functions_exist|test_store_functions_no_domain_validation|test_state_version_owned_by_apply_mcp_command|test_provenance_owned_by_engine_action|test_no_ettle_delete_variant|test_ettle_handler_no_raw_sql/)'

test-slice:
	cargo nextest run --workspace $(SLICE_TEST_FILTER)

test-full:
	cargo nextest run --workspace

test:
	cargo nextest run --workspace

build:
	cargo build --workspace --target aarch64-apple-darwin

clean:
	cargo clean

coverage-check:
	@echo "Running tests with coverage (minimum threshold: $(COVERAGE_MIN)%)..."
	@mkdir -p coverage
	@cargo llvm-cov nextest --workspace --cobertura --output-path coverage/cobertura.xml
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
	@mkdir -p coverage
	@cargo llvm-cov nextest --workspace --html --output-dir coverage
	@echo "✅ HTML coverage report generated: coverage/html/index.html"
	@echo "Open with: open coverage/html/index.html"
